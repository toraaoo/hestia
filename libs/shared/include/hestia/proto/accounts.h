#pragma once

#include <cstdint>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

#include <nlohmann/json.hpp>

#include <hestia/proto/contract.h>

// The accounts domain: Minecraft accounts signed in through Microsoft. Tokens
// never cross the wire — the daemon keeps them; clients see uuid and name.
namespace hestia::proto {
    enum class LoginMethod : std::uint8_t { device_code, sisu };

    std::optional<LoginMethod> parse_login_method(std::string_view name);
    const char *to_string(LoginMethod method);
    void to_json(nlohmann::json &j, LoginMethod method);
    void from_json(const nlohmann::json &j, LoginMethod &method);

    struct Account {
        std::string uuid;
        std::string name;

        static constexpr auto kFields = fields(field("uuid", &Account::uuid), field("name", &Account::name));
    };

    struct AccountLoginBegin {
        static constexpr const char *kChannel = "account.login.begin";
        struct Params {
            LoginMethod method = LoginMethod::device_code;

            static constexpr auto kFields = fields(field("method", &Params::method));
        };
        struct Result {
            std::string id;
            LoginMethod method = LoginMethod::device_code;
            std::string url;
            std::string user_code;
            std::string verification_uri;

            static constexpr auto kFields =
                fields(field("id", &Result::id), field("method", &Result::method),
                       field("url", &Result::url, kOmitIfEmpty),
                       field("user_code", &Result::user_code, kOmitIfEmpty),
                       field("verification_uri", &Result::verification_uri, kOmitIfEmpty));
        };
    };

    struct AccountLoginComplete {
        static constexpr const char *kChannel = "account.login.complete";
        struct Params {
            std::string id;
            std::string code;

            static constexpr auto kFields =
                fields(field("id", &Params::id, kRequired), field("code", &Params::code, kOmitIfEmpty));
        };
        struct Result {
            Account account;

            static constexpr auto kFields = fields(field("account", &Result::account));
        };
    };

    struct AccountList {
        static constexpr const char *kChannel = "account.list";
        using Params = Empty;
        struct Result {
            std::vector<Account> accounts;

            static constexpr auto kFields = fields(field("accounts", &Result::accounts));
        };
    };

    struct AccountRemove {
        static constexpr const char *kChannel = "account.remove";
        struct Params {
            std::string account; // name or uuid

            static constexpr auto kFields = fields(field("account", &Params::account, kRequired));
        };
        using Result = Empty;
    };
} // namespace hestia::proto
