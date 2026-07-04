#include "hestia/client/accounts.h"

#include "session.h"

namespace hestia::client {
    proto::Account Accounts::login(const AccountLoginCodeCallback &on_code) {
        const auto id = job_id("login");
        const auto done = session_->run_job(
            id, proto::AccountLoginDoneEvent::kTopic, proto::AccountLoginErrorEvent::kTopic,
            [&on_code](const ipc::Event &event) {
                if (event.topic != proto::AccountLoginCodeEvent::kTopic || !on_code) return;
                on_code(event.payload.get<proto::AccountLoginCodeEvent>().code);
            },
            [&] { session_->call<proto::AccountLogin>({.id = id}); });
        return done.get<proto::AccountLoginDoneEvent>().account;
    }

    std::vector<proto::Account> Accounts::list() {
        return session_->call<proto::AccountList>().accounts;
    }

    void Accounts::remove(const std::string &ref) {
        session_->call<proto::AccountRemove>({.account = ref});
    }
} // namespace hestia::client
