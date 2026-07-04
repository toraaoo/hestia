#pragma once

#include <string>
#include <vector>

#include <hestia/proto/contract.h>

// The accounts domain: Minecraft accounts signed in through Microsoft. Tokens
// never cross the wire — the daemon keeps them; clients see uuid and name.
namespace hestia::proto {
    struct Account {
        std::string uuid;
        std::string name;

        static constexpr auto kFields = fields(field("uuid", &Account::uuid), field("name", &Account::name));
    };

    struct AccountLoginCode {
        std::string user_code;
        std::string verification_uri;
        int expires_in = 0; // seconds until the code stops working

        static constexpr auto kFields = fields(field("user_code", &AccountLoginCode::user_code),
                                               field("verification_uri", &AccountLoginCode::verification_uri),
                                               field("expires_in", &AccountLoginCode::expires_in));
    };

    struct AccountLogin {
        static constexpr const char *kChannel = "account.login";
        struct Params {
            std::string id; // caller-assigned job id; empty lets the daemon generate one

            static constexpr auto kFields = fields(field("id", &Params::id));
        };
        struct Result {
            std::string id;

            static constexpr auto kFields = fields(field("id", &Result::id));
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

    struct AccountLoginCodeEvent {
        static constexpr const char *kTopic = "account.login.code";
        std::string id;
        AccountLoginCode code;

        static constexpr auto kFields =
            fields(field("id", &AccountLoginCodeEvent::id), field("", &AccountLoginCodeEvent::code, kFlatten));
    };

    struct AccountLoginDoneEvent {
        static constexpr const char *kTopic = "account.login.done";
        std::string id;
        Account account;

        static constexpr auto kFields =
            fields(field("id", &AccountLoginDoneEvent::id), field("account", &AccountLoginDoneEvent::account));
    };

    struct AccountLoginErrorEvent {
        static constexpr const char *kTopic = "account.login.error";
        std::string id;
        std::string message;

        static constexpr auto kFields =
            fields(field("id", &AccountLoginErrorEvent::id), field("message", &AccountLoginErrorEvent::message));
    };
} // namespace hestia::proto
