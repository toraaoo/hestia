#include "services/accounts_service.h"

#include "runtime/channels.h"
#include "runtime/runtime.h"

#include <hestia/engine/engine.h>
#include <hestia/proto/accounts.h>

namespace hestia::daemon {
    void AccountsService::register_channels(Channels &on) {
        on.handle<proto::AccountLoginBegin>([](const proto::AccountLoginBegin::Params &p, HandlerContext &ctx) {
            const auto challenge = ctx.runtime.engine().accounts().begin_login(p.method);
            return proto::AccountLoginBegin::Result{.id = challenge.id,
                                                    .method = challenge.method,
                                                    .url = challenge.url,
                                                    .user_code = challenge.user_code,
                                                    .verification_uri = challenge.verification_uri};
        });

        on.handle<proto::AccountLoginComplete>([](const proto::AccountLoginComplete::Params &p, HandlerContext &ctx) {
            return proto::AccountLoginComplete::Result{
                .account = ctx.runtime.engine().accounts().complete_login(p.id, p.code)};
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
