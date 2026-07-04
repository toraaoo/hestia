#pragma once

#include <string>
#include <vector>

#include <hestia/client/facade.h>
#include <hestia/proto/accounts.h>

namespace hestia::client {
    class Accounts : public Facade {
    public:
        using Facade::Facade;

        proto::AccountLoginBegin::Result begin_login(proto::LoginMethod method = proto::LoginMethod::device_code);
        proto::Account complete_login(const std::string &id, const std::string &code = {});

        std::vector<proto::Account> list();

        void remove(const std::string &ref);
    };
} // namespace hestia::client
