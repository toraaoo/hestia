#pragma once

#include <functional>
#include <string>
#include <vector>

#include <hestia/client/facade.h>
#include <hestia/proto/accounts.h>

namespace hestia::client {
    using AccountLoginCodeCallback = std::function<void(const proto::AccountLoginCode &)>;

    // Minecraft accounts, signed in through Microsoft by the daemon.
    class Accounts : public Facade {
    public:
        using Facade::Facade;

        // Blocks until the sign-in completes: `on_code` receives the code the
        // user must enter at the verification URL (reported on the reader
        // thread); like Java::install(), it uses the session's single
        // event-callback slot.
        proto::Account login(const AccountLoginCodeCallback &on_code);

        std::vector<proto::Account> list();

        // Removes the account whose name or uuid matches `ref`; throws when
        // nothing matches.
        void remove(const std::string &ref);
    };
} // namespace hestia::client
