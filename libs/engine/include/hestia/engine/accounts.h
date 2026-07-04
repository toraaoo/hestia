#pragma once

#include <filesystem>
#include <map>
#include <memory>
#include <mutex>
#include <string>
#include <vector>

#include <hestia/proto/accounts.h>

namespace hestia::engine {
    struct LoginChallenge {
        std::string id;
        std::string url;
    };

    struct LoginSession;

    class Accounts {
    public:
        explicit Accounts(std::filesystem::path path);
        ~Accounts();

        [[nodiscard]] std::vector<proto::Account> list() const;

        LoginChallenge begin_login();
        proto::Account complete_login(const std::string &id, const std::string &code);

        // A currently-valid Minecraft access token for the account named by `ref`
        // (uuid or name), rotating the stored tokens through Microsoft when they
        // are at or near expiry. Throws if nothing matches or the refresh fails.
        std::string access_token(const std::string &ref);

        bool remove(const std::string &ref);

        void reload(std::filesystem::path path);

    private:
        mutable std::mutex mu_;
        std::filesystem::path path_;
        std::map<std::string, std::unique_ptr<LoginSession>> pending_;
    };
} // namespace hestia::engine
