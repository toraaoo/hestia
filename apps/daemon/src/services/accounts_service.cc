#include "services/accounts_service.h"

#include "accounts/login_manager.h"
#include "runtime/channels.h"
#include "runtime/runtime.h"

#include <hestia/engine/engine.h>
#include <hestia/proto/accounts.h>

namespace hestia::daemon {
    void AccountsService::register_channels(Channels &on) {
        on.handle<proto::AccountLogin>([](const proto::AccountLogin::Params &p, HandlerContext &ctx) {
            const auto id = ctx.runtime.logins().start(p.id);
            if (!id) {
                throw ServiceError(ipc::errors::kBadRequest, "a sign-in is already in progress");
            }
            return proto::AccountLogin::Result{.id = *id};
        });

        on.handle<proto::AccountList>([](const proto::Empty &, HandlerContext &ctx) {
            return proto::AccountList::Result{.accounts = ctx.runtime.engine().accounts().list()};
        });

        on.handle<proto::AccountRemove>([](const proto::AccountRemove::Params &p, HandlerContext &ctx) {
            if (!ctx.runtime.engine().accounts().remove(p.account)) {
                throw ServiceError(ipc::errors::kNotFound, "no account matches '" + p.account + "'");
            }
            return proto::Empty{};
        });
    }
} // namespace hestia::daemon
