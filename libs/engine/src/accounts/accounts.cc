#include <hestia/engine/accounts.h>

#include <chrono>
#include <fstream>
#include <stdexcept>
#include <utility>
#include <vector>

#include "accounts/microsoft.h"

namespace hestia::engine {
    namespace {
        // The ADL hooks nlohmann needs for the reflected types below.
        using proto::from_json; // NOLINT(misc-unused-using-decls)
        using proto::to_json;   // NOLINT(misc-unused-using-decls)

        struct StoredAccount {
            std::string uuid;
            std::string name;
            std::string refresh_token;
            std::string access_token;
            long long expires_at = 0; // unix seconds

            static constexpr auto kFields = proto::fields(
                proto::field("uuid", &StoredAccount::uuid, proto::kRequired),
                proto::field("name", &StoredAccount::name, proto::kRequired),
                proto::field("refresh_token", &StoredAccount::refresh_token),
                proto::field("access_token", &StoredAccount::access_token),
                proto::field("expires_at", &StoredAccount::expires_at));
        };

        struct AccountsFile {
            std::vector<StoredAccount> accounts;

            static constexpr auto kFields = proto::fields(proto::field("accounts", &AccountsFile::accounts));
        };

        AccountsFile load(const std::filesystem::path &path) {
            std::ifstream in(path);
            if (!in) return {};
            const auto doc = nlohmann::json::parse(in, nullptr, false);
            if (!doc.is_object()) return {};
            try {
                return doc.get<AccountsFile>();
            } catch (const std::exception &) {
                return {};
            }
        }

        void save(const std::filesystem::path &path, const AccountsFile &file) {
            if (path.has_parent_path()) {
                std::filesystem::create_directories(path.parent_path());
            }
            std::ofstream out(path, std::ios::trunc);
            if (!out) {
                throw std::runtime_error("failed to open accounts file for writing: " + path.string());
            }
            out << nlohmann::json(file).dump(2) << '\n';
            out.close();
            // The file holds tokens: keep it owner-only where permissions exist.
            std::error_code ec;
            std::filesystem::permissions(path,
                                         std::filesystem::perms::owner_read | std::filesystem::perms::owner_write,
                                         std::filesystem::perm_options::replace, ec);
        }
    } // namespace

    Accounts::Accounts(std::filesystem::path path) : path_(std::move(path)) {}

    std::vector<proto::Account> Accounts::list() const {
        std::scoped_lock const lk(mu_);
        std::vector<proto::Account> accounts;
        for (const auto &stored: load(path_).accounts) {
            accounts.push_back(proto::Account{.uuid = stored.uuid, .name = stored.name});
        }
        return accounts;
    }

    proto::Account Accounts::login(const std::string &client_id, const DeviceCodeCallback &on_code,
                                   const std::function<bool()> &cancelled) {
        const auto authorization = msa_request_device_code(client_id);
        if (on_code) on_code(authorization.code);
        const auto msa = msa_poll_for_tokens(client_id, authorization, cancelled);
        const auto minecraft = minecraft_login(msa.access_token);
        const auto profile = minecraft_profile(minecraft.access_token);

        const auto now = std::chrono::duration_cast<std::chrono::seconds>(
                             std::chrono::system_clock::now().time_since_epoch())
                             .count();
        StoredAccount record{.uuid = profile.uuid,
                             .name = profile.name,
                             .refresh_token = msa.refresh_token,
                             .access_token = minecraft.access_token,
                             .expires_at = now + minecraft.expires_in};

        std::scoped_lock const lk(mu_);
        auto file = load(path_);
        std::erase_if(file.accounts, [&](const StoredAccount &a) { return a.uuid == record.uuid; });
        file.accounts.push_back(std::move(record));
        save(path_, file);
        return proto::Account{.uuid = profile.uuid, .name = profile.name};
    }

    bool Accounts::remove(const std::string &ref) {
        std::scoped_lock const lk(mu_);
        auto file = load(path_);
        const auto removed =
            std::erase_if(file.accounts, [&](const StoredAccount &a) { return a.uuid == ref || a.name == ref; });
        if (removed == 0) return false;
        save(path_, file);
        return true;
    }

    void Accounts::reload(std::filesystem::path path) {
        std::scoped_lock const lk(mu_);
        path_ = std::move(path);
    }
} // namespace hestia::engine
