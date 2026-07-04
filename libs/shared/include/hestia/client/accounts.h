#pragma once

#include <string>
#include <vector>

#include <hestia/client/facade.h>
#include <hestia/proto/accounts.h>

namespace hestia::client {
    // Minecraft accounts, signed in through Microsoft by the daemon. Sign-in is
    // two calls: begin_login() returns the URL the user opens; after they sign in
    // and are redirected, complete_login() redeems the code from that redirect.
    class Accounts : public Facade {
    public:
        using Facade::Facade;

        proto::AccountLoginBegin::Result begin_login();
        proto::Account complete_login(const std::string &id, const std::string &code);

        std::vector<proto::Account> list();

        void remove(const std::string &ref);
    };
} // namespace hestia::client
