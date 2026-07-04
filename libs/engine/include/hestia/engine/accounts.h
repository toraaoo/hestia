#pragma once

#include <filesystem>
#include <functional>
#include <mutex>
#include <string>
#include <vector>

#include <hestia/proto/accounts.h>

namespace hestia::engine {
    using DeviceCodeCallback = std::function<void(const proto::AccountLoginCode &)>;

    // Minecraft accounts signed in through Microsoft, persisted with their
    // tokens in <data_home>/accounts.json (owner-only on POSIX).
    class Accounts {
    public:
        explicit Accounts(std::filesystem::path path);

        [[nodiscard]] std::vector<proto::Account> list() const;

        // Blocking Microsoft device-code login: request a code (reported through
        // on_code), poll until the user approves in their browser, then walk
        // Xbox Live -> XSTS -> Minecraft services, fetch the profile, and
        // persist the account. `cancelled` is polled between attempts so a
        // daemon shutdown never waits out the code's lifetime.
        proto::Account login(const std::string &client_id, const DeviceCodeCallback &on_code,
                             const std::function<bool()> &cancelled = {});

        // Removes the account whose name or uuid matches `ref`.
        bool remove(const std::string &ref);

        void reload(std::filesystem::path path);

    private:
        mutable std::mutex mu_;
        std::filesystem::path path_;
    };
} // namespace hestia::engine
