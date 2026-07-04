#include <hestia/engine/java.h>

#include <algorithm>
#include <fstream>
#include <stdexcept>
#include <utility>

#include <fmt/format.h>
#include <nlohmann/json.hpp>
#include <spdlog/spdlog.h>

#include <hestia/engine/downloader.h>

#include "java/adoptium_provider.h"
#include "java/extract.h"
#include "java/platform.h"

namespace hestia::engine {
    namespace fs = std::filesystem;
    using nlohmann::json;

    namespace {
        constexpr const char *kRuntimeRecord = "runtime.json";

        // Remote catalogue data must stay a plain file name before it is joined
        // onto a local path.
        void validate_archive_name(const std::string &name) {
            if (name.empty() || name.front() == '.' || name.find('/') != std::string::npos ||
                name.find('\\') != std::string::npos || name.find('"') != std::string::npos) {
                throw std::runtime_error(fmt::format("provider returned an unsafe archive name: '{}'", name));
            }
        }

        void write_runtime_record(const fs::path &install_dir, const JavaPackage &package,
                                  const fs::path &relative_executable) {
            const json record{
                {"vendor", package.vendor},
                {"major", package.major},
                {"release_name", package.release_name},
                {"executable", relative_executable.generic_string()},
            };
            std::ofstream out(install_dir / kRuntimeRecord, std::ios::trunc);
            if (!(out << record.dump(2) << '\n')) {
                throw std::runtime_error(fmt::format("cannot write {}", (install_dir / kRuntimeRecord).string()));
            }
        }

        std::optional<proto::JavaRuntime> read_runtime(const fs::path &install_dir) {
            std::ifstream in(install_dir / kRuntimeRecord);
            if (!in) return std::nullopt;
            json record;
            try {
                in >> record;
            } catch (const json::exception &) {
                return std::nullopt;
            }
            proto::JavaRuntime runtime;
            runtime.vendor = record.value("vendor", std::string{});
            runtime.major = record.value("major", 0);
            runtime.release_name = record.value("release_name", std::string{});
            runtime.executable = install_dir / fs::path(record.value("executable", std::string{}));
            runtime.home = runtime.executable.parent_path().parent_path();
            std::error_code ec;
            if (runtime.major <= 0 || !fs::is_regular_file(runtime.executable, ec)) return std::nullopt;
            return runtime;
        }

        void remove_quietly(const fs::path &path) {
            std::error_code ec;
            fs::remove_all(path, ec);
        }
    } // namespace

    Java::Java(fs::path dir, Cache *cache) : dir_(std::move(dir)), cache_(cache) {
        providers_.push_back(std::make_unique<AdoptiumProvider>());
    }

    Java::Java(fs::path dir, std::vector<std::unique_ptr<JavaProvider>> providers, Cache *cache)
        : dir_(std::move(dir)), providers_(std::move(providers)), cache_(cache) {
        if (providers_.empty()) {
            throw std::invalid_argument("Java requires at least one provider");
        }
    }

    const JavaProvider &Java::provider() const {
        return *providers_.front();
    }

    fs::path Java::dir() const {
        std::scoped_lock const lk(mu_);
        return dir_;
    }

    void Java::reload(fs::path dir) {
        std::scoped_lock const lk(mu_);
        dir_ = std::move(dir);
    }

    std::vector<proto::JavaRelease> Java::releases() const {
        return provider().releases();
    }

    std::vector<proto::JavaRuntime> Java::installed() const {
        std::vector<proto::JavaRuntime> runtimes;
        std::error_code ec;
        for (const auto &entry: fs::directory_iterator(dir(), ec)) {
            if (!entry.is_directory(ec)) continue;
            if (auto runtime = read_runtime(entry.path())) {
                runtimes.push_back(std::move(*runtime));
            }
        }
        std::ranges::sort(runtimes, {}, &proto::JavaRuntime::major);
        return runtimes;
    }

    JavaInstallOutcome Java::install(int major, bool force, const JavaInstallProgressCallback &on_progress) {
        if (major <= 0) {
            throw std::runtime_error(fmt::format("invalid java major version: {}", major));
        }
        if (!force) {
            for (auto &runtime: installed()) {
                if (runtime.major == major) {
                    spdlog::debug("java {} already installed ({}), skipping", major, runtime.release_name);
                    return {.runtime = std::move(runtime), .already_installed = true};
                }
            }
        }
        const auto report = [&](proto::JavaInstallPhase phase) {
            if (on_progress) on_progress(proto::JavaInstallProgress{.phase = phase});
        };

        spdlog::info("installing java {}{}", major, force ? " (forced)" : "");
        report(proto::JavaInstallPhase::resolving);
        const JavaPackage package = provider().resolve(major, host_target());
        validate_archive_name(package.archive_name);
        spdlog::debug("resolved java {} to {} ({})", major, package.release_name, package.url);

        const fs::path base = dir();
        const fs::path install_dir = base / fmt::format("{}-{}", package.vendor, package.major);
        const fs::path archive = base / "tmp" / package.archive_name;
        const fs::path staging = install_dir.string() + ".staging";

        remove_quietly(staging);
        try {
            Downloader{cache_}.fetch(package.url, archive, package.checksum, [&](const proto::DownloadProgress &progress) {
                if (on_progress) {
                    on_progress(proto::JavaInstallProgress{.phase = proto::JavaInstallPhase::downloading,
                                                         .current = progress.downloaded,
                                                         .total = progress.total});
                }
            });

            spdlog::debug("extracting {} into {}", package.archive_name, staging.string());
            report(proto::JavaInstallPhase::extracting);
            extract_archive(archive, staging, [&](std::uint64_t done, std::uint64_t total) {
                if (on_progress) {
                    on_progress(proto::JavaInstallProgress{
                        .phase = proto::JavaInstallPhase::extracting, .current = done, .total = total});
                }
            });

            const auto executable = find_java_executable(staging);
            if (!executable) {
                throw std::runtime_error(
                    fmt::format("archive {} contained no java executable", package.archive_name));
            }
            write_runtime_record(staging, package, fs::relative(*executable, staging));

            remove_quietly(install_dir);
            fs::rename(staging, install_dir);
        } catch (...) {
            remove_quietly(staging);
            remove_quietly(archive);
            throw;
        }
        remove_quietly(archive);

        auto runtime = read_runtime(install_dir);
        if (!runtime) {
            throw std::runtime_error(fmt::format("install of {} did not produce a usable runtime",
                                                 package.release_name));
        }
        spdlog::info("installed java {} ({}) at {}", major, package.release_name, install_dir.string());
        return {.runtime = std::move(*runtime), .already_installed = false};
    }

    bool Java::uninstall(int major) {
        bool removed = false;
        std::error_code ec;
        for (const auto &entry: fs::directory_iterator(dir(), ec)) {
            if (!entry.is_directory(ec)) continue;
            const auto runtime = read_runtime(entry.path());
            if (runtime && runtime->major == major) {
                fs::remove_all(entry.path());
                removed = true;
            }
        }
        if (removed) {
            spdlog::info("uninstalled java {}", major);
        } else {
            spdlog::debug("uninstall java {}: nothing installed", major);
        }
        return removed;
    }
} // namespace hestia::engine
