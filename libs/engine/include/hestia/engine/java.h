#pragma once

#include <filesystem>
#include <functional>
#include <memory>
#include <mutex>
#include <string>
#include <vector>

#include <hestia/proto/download.h>
#include <hestia/proto/java.h>

namespace hestia::engine {
    // Adoptium's platform vocabulary: "linux"/"mac"/"windows", "x64"/"aarch64".
    struct JavaTarget {
        std::string os;
        std::string arch;
    };

    JavaTarget host_target();

    struct JavaPackage {
        std::string vendor;
        int major = 0;
        std::string release_name;
        std::string url;
        std::string archive_name;
        proto::Checksum checksum;
    };

    // One vendor's release catalogue. Implementations fetch metadata only —
    // downloading, extraction, and registration live in Java.
    class JavaProvider {
    public:
        virtual ~JavaProvider() = default;

        [[nodiscard]] virtual std::string vendor() const = 0;

        [[nodiscard]] virtual std::vector<proto::JavaRelease> releases() const = 0;

        // The latest GA build of `major` for `target`; throws when there is none.
        [[nodiscard]] virtual JavaPackage resolve(int major, const JavaTarget &target) const = 0;
    };

    using JavaInstallProgressCallback = std::function<void(const proto::JavaInstallProgress &)>;

    struct JavaInstallOutcome {
        proto::JavaRuntime runtime;
        bool already_installed = false;
    };

    // Installs and tracks Java runtimes: each install lives at
    // <dir>/<vendor>-<major>/ beside a runtime.json record, and listing scans
    // that directory — the disk is the registry.
    class Cache;

    class Java {
    public:
        explicit Java(std::filesystem::path dir, Cache *cache = nullptr); // default provider (Adoptium)

        Java(std::filesystem::path dir, std::vector<std::unique_ptr<JavaProvider>> providers, Cache *cache = nullptr);

        [[nodiscard]] std::vector<proto::JavaRelease> releases() const;

        [[nodiscard]] std::vector<proto::JavaRuntime> installed() const;

        // Blocking resolve → download → extract → register; a failed install
        // leaves nothing behind. An already-installed line is returned as-is
        // unless `force`, which reinstalls over it.
        JavaInstallOutcome install(int major, bool force = false, const JavaInstallProgressCallback &on_progress = {});

        bool uninstall(int major);

        void reload(std::filesystem::path dir);

    private:
        const JavaProvider &provider() const;
        std::filesystem::path dir() const;

        mutable std::mutex mu_;
        std::filesystem::path dir_;
        std::vector<std::unique_ptr<JavaProvider>> providers_;
        Cache *cache_;
    };
} // namespace hestia::engine
