#include <hestia/engine/accounts.h>

#include <algorithm>
#include <chrono>
#include <fstream>
#include <stdexcept>
#include <utility>
#include <vector>

#include "accounts/microsoft.h"
#include "accounts/signing.h"
#include "download/checksum.h"

namespace hestia::engine {
    struct LoginSession {
        ProofKey key;
        std::string device_token;
        std::string verifier;
        std::string session_id;
    };

    namespace {
        std::string to_hex(const std::vector<std::uint8_t> &bytes) {
            constexpr char digits[] = "0123456789abcdef";
            std::string out;
            out.reserve(bytes.size() * 2);
            for (const auto byte: bytes) {
                out.push_back(digits[byte >> 4]);
                out.push_back(digits[byte & 0x0F]);
            }
            return out;
        }

        std::vector<std::uint8_t> sha256_bytes(const std::string &text) {
            Hasher hasher(proto::HashAlgorithm::sha256);
            hasher.update(text.data(), text.size());
            const auto hex = hasher.hex_digest();
            std::vector<std::uint8_t> out(hex.size() / 2);
            for (std::size_t i = 0; i < out.size(); ++i) {
                out[i] = static_cast<std::uint8_t>(std::stoul(hex.substr(i * 2, 2), nullptr, 16));
            }
            return out;
        }
    } // namespace

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

        constexpr long long kRefreshMarginSeconds = 300;

        long long now_seconds() {
            return std::chrono::duration_cast<std::chrono::seconds>(
                       std::chrono::system_clock::now().time_since_epoch())
                .count();
        }

        void rotate_tokens(StoredAccount &account) {
            if (account.refresh_token.empty()) {
                throw std::runtime_error("this account has no refresh token; sign in again");
            }
            const auto oauth = refresh_oauth(account.refresh_token);
            auto key = ProofKey::generate();
            const auto device_token = request_device_token(key);
            const auto authorization = sisu_authorize("", oauth.access_token, device_token, key);
            const auto xsts = xsts_authorize(authorization, device_token, key);

            account.access_token = launcher_login(xsts);
            if (!oauth.refresh_token.empty()) account.refresh_token = oauth.refresh_token;
            account.expires_at = now_seconds() + oauth.expires_in;
        }
    } // namespace

    Accounts::Accounts(std::filesystem::path path) : path_(std::move(path)) {}
    Accounts::~Accounts() = default;

    std::vector<proto::Account> Accounts::list() const {
        std::scoped_lock const lk(mu_);
        std::vector<proto::Account> accounts;
        for (const auto &stored: load(path_).accounts) {
            accounts.push_back(proto::Account{.uuid = stored.uuid, .name = stored.name});
        }
        return accounts;
    }

    LoginChallenge Accounts::begin_login() {
        auto key = ProofKey::generate();
        auto device_token = request_device_token(key);
        auto verifier = to_hex(random_bytes(64));

        const auto challenge = base64url_nopad(sha256_bytes(verifier));
        const auto state = to_hex(random_bytes(16));
        const auto auth = sisu_authenticate(device_token, challenge, state, key);

        auto session = std::unique_ptr<LoginSession>(new LoginSession{.key = std::move(key),
                                                                      .device_token = std::move(device_token),
                                                                      .verifier = std::move(verifier),
                                                                      .session_id = auth.session_id});

        auto id = format_uuid_v4(random_bytes(16));
        std::scoped_lock const lk(mu_);
        pending_[id] = std::move(session);
        return LoginChallenge{.id = id, .url = auth.url};
    }

    proto::Account Accounts::complete_login(const std::string &id, const std::string &code) {
        std::unique_ptr<LoginSession> session;
        {
            std::scoped_lock const lk(mu_);
            const auto it = pending_.find(id);
            if (it == pending_.end()) {
                throw std::runtime_error("no sign-in is in progress for this request");
            }
            session = std::move(it->second);
            pending_.erase(it);
        }

        const auto oauth = redeem_code(code, session->verifier);
        const auto authorization =
            sisu_authorize(session->session_id, oauth.access_token, session->device_token, session->key);
        const auto xsts = xsts_authorize(authorization, session->device_token, session->key);
        const auto minecraft_token = launcher_login(xsts);
        const auto profile = minecraft_profile(minecraft_token);

        const auto now = std::chrono::duration_cast<std::chrono::seconds>(
                             std::chrono::system_clock::now().time_since_epoch())
                             .count();
        StoredAccount record{.uuid = profile.uuid,
                             .name = profile.name,
                             .refresh_token = oauth.refresh_token,
                             .access_token = minecraft_token,
                             .expires_at = now + oauth.expires_in};

        std::scoped_lock const lk(mu_);
        auto file = load(path_);
        std::erase_if(file.accounts, [&](const StoredAccount &a) { return a.uuid == record.uuid; });
        file.accounts.push_back(std::move(record));
        save(path_, file);
        return proto::Account{.uuid = profile.uuid, .name = profile.name};
    }

    std::string Accounts::access_token(const std::string &ref) {
        std::scoped_lock const lk(mu_);
        auto file = load(path_);
        const auto it = std::find_if(file.accounts.begin(), file.accounts.end(),
                                     [&](const StoredAccount &a) { return a.uuid == ref || a.name == ref; });
        if (it == file.accounts.end()) {
            throw std::runtime_error("no account matches '" + ref + "'");
        }
        if (it->expires_at - now_seconds() <= kRefreshMarginSeconds) {
            rotate_tokens(*it);
            save(path_, file);
        }
        return it->access_token;
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
