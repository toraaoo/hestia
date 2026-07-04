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

    struct AccountLoginBegin {
        static constexpr const char *kChannel = "account.login.begin";
        using Params = Empty;
        struct Result {
            std::string id;
            std::string url;

            static constexpr auto kFields = fields(field("id", &Result::id), field("url", &Result::url));
        };
    };

    struct AccountLoginComplete {
        static constexpr const char *kChannel = "account.login.complete";
        struct Params {
            std::string id;
            std::string code;

            static constexpr auto kFields =
                fields(field("id", &Params::id, kRequired), field("code", &Params::code, kRequired));
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
