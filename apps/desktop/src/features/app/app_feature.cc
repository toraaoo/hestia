#include "features/app/app_feature.h"
#include "core/ipc/ipc_router.h"
#include "core/build_config.h"
#include <hestia/greeting.h>

namespace desktop::features {

void AppFeature::RegisterActions(ipc::Actions& on) {
    on("info", [](const ipc::Request&, ipc::Response res) {
        auto d = CefDictionaryValue::Create();
        d->SetString("name",    APP_NAME);
        d->SetString("id",      APP_ID);
        d->SetString("vendor",  APP_VENDOR);
        d->SetString("version", APP_VERSION);
        d->SetString("channel", APP_CHANNEL);
        d->SetString("scheme",  APP_SCHEME);
        d->SetString("platform", APP_PLATFORM);
        res.Success(ipc::Dict(d));
    });

    on("ping", [](const ipc::Request& req, ipc::Response res) {
        const auto msg = req.PayloadString();
        res.Success(ipc::Str(msg.empty() ? "pong" : msg));
    });

    on("greet", [](const ipc::Request& req, ipc::Response res) {
        res.Success(ipc::Str(hestia::greeting::greet(req.PayloadString())));
    });
}

}  // namespace desktop::features
