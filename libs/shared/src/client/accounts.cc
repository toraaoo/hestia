#include "hestia/client/accounts.h"

#include <chrono>

#include "session.h"

namespace hestia::client {
    proto::AccountLoginBegin::Result Accounts::begin_login() {
        return session_->call<proto::AccountLoginBegin>({}, std::chrono::seconds(60));
    }

    proto::Account Accounts::complete_login(const std::string &id, const std::string &code) {
        return session_->call<proto::AccountLoginComplete>({.id = id, .code = code}, std::chrono::seconds(120))
            .account;
    }

    std::vector<proto::Account> Accounts::list() {
        return session_->call<proto::AccountList>().accounts;
    }

    void Accounts::remove(const std::string &ref) {
        session_->call<proto::AccountRemove>({.account = ref});
    }
} // namespace hestia::client
