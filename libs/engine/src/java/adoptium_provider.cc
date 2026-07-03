#include "java/adoptium_provider.h"

#include <algorithm>
#include <stdexcept>

#include <cpr/cpr.h>
#include <fmt/format.h>

namespace hestia::engine {
    using nlohmann::json;

    namespace {
        constexpr const char *kApiBase = "https://api.adoptium.net";

        json fetch_json(const std::string &url, cpr::Parameters parameters = {}) {
            const cpr::Response response = cpr::Get(cpr::Url{url}, std::move(parameters));
            if (response.error) {
                throw std::runtime_error(fmt::format("adoptium request failed: {}", response.error.message));
            }
            if (response.status_code != 200) {
                throw std::runtime_error(fmt::format("adoptium request failed: HTTP {}", response.status_code));
            }
            try {
                return json::parse(response.text);
            } catch (const json::exception &e) {
                throw std::runtime_error(fmt::format("adoptium returned malformed JSON: {}", e.what()));
            }
        }
    } // namespace

    std::vector<ipc::JavaRelease> adoptium_releases_from_json(const json &j) {
        if (!j.contains("available_releases") || !j["available_releases"].is_array()) {
            throw std::runtime_error("adoptium response is missing available_releases");
        }
        const auto lts = j.value("available_lts_releases", json::array());
        std::vector<ipc::JavaRelease> releases;
        for (const auto &major: j["available_releases"]) {
            if (!major.is_number_integer()) continue;
            releases.push_back(ipc::JavaRelease{
                .major = major.get<int>(),
                .lts = std::ranges::find(lts, major) != lts.end(),
            });
        }
        std::ranges::sort(releases, {}, &ipc::JavaRelease::major);
        return releases;
    }

    JavaPackage adoptium_package_from_json(const json &assets, int major, const JavaTarget &target) {
        if (!assets.is_array()) {
            throw std::runtime_error("adoptium assets response is not an array");
        }
        for (const auto &asset: assets) {
            const auto &binary = asset.value("binary", json::object());
            if (binary.value("os", std::string{}) != target.os) continue;
            if (binary.value("architecture", std::string{}) != target.arch) continue;
            if (binary.value("image_type", std::string{}) != "jdk") continue;

            const auto &package = binary.value("package", json::object());
            JavaPackage resolved{
                .vendor = "temurin",
                .major = major,
                .release_name = asset.value("release_name", std::string{}),
                .url = package.value("link", std::string{}),
                .archive_name = package.value("name", std::string{}),
                .checksum = ipc::Checksum{.algorithm = ipc::HashAlgorithm::sha256,
                                          .hex = package.value("checksum", std::string{})},
            };
            if (resolved.url.empty() || resolved.archive_name.empty() ||
                !ipc::is_valid_checksum(resolved.checksum)) {
                throw std::runtime_error(
                    fmt::format("adoptium build for temurin {} is missing its download link or checksum", major));
            }
            return resolved;
        }
        throw std::runtime_error(
            fmt::format("no temurin {} jdk build is published for {}/{}", major, target.os, target.arch));
    }

    std::vector<ipc::JavaRelease> AdoptiumProvider::releases() const {
        return adoptium_releases_from_json(fetch_json(fmt::format("{}/v3/info/available_releases", kApiBase)));
    }

    JavaPackage AdoptiumProvider::resolve(int major, const JavaTarget &target) const {
        const auto assets = fetch_json(fmt::format("{}/v3/assets/latest/{}/hotspot", kApiBase, major),
                                       cpr::Parameters{{"os", target.os},
                                                       {"architecture", target.arch},
                                                       {"image_type", "jdk"},
                                                       {"vendor", "eclipse"}});
        return adoptium_package_from_json(assets, major, target);
    }
} // namespace hestia::engine
