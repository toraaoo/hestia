#include "features/app/app_feature.h"
#include "core/ipc/ipc_router.h"
#include "core/build_config.h"
#include "core/daemon/daemon_client.h"
#include "core/window/window_util.h"
#include "core/browser/client_manager.h"

namespace desktop::features {

namespace {

void EmitWindowState(CefRefPtr<CefWindow> win) {
    if (!win) return;
    auto browser = browser::ClientManager::Instance().GetMainBrowser();
    if (!browser) return;
    auto d = CefDictionaryValue::Create();
    d->SetBool("maximized", win->IsMaximized());
    d->SetBool("minimized", window::IsMinimized());
    ipc::Emit(browser, "app.window.state", ipc::Dict(d));
}

std::string WindowStateJson(CefRefPtr<CefWindow> win) {
    auto d = CefDictionaryValue::Create();
    d->SetBool("maximized", win ? win->IsMaximized() : false);
    d->SetBool("minimized", window::IsMinimized());
    return ipc::Dict(d);
}

}  // namespace

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

    RegisterForward(on, "greet", "app.greet");

    on("window.state", [](const ipc::Request&, ipc::Response res) {
        res.Success(WindowStateJson(window::GetActiveWindow()));
    });

    on("window.minimize", [](const ipc::Request&, ipc::Response res) {
        auto win = window::GetActiveWindow();
        if (!win) { res.Failure(-1, "no window"); return; }
        window::SetMinimized(true);
        win->Minimize();
        EmitWindowState(win);
        res.Success(ipc::Null());
    });

    on("window.maximize", [](const ipc::Request&, ipc::Response res) {
        auto win = window::GetActiveWindow();
        if (!win) { res.Failure(-1, "no window"); return; }
        if (win->IsMaximized()) {
            win->Restore();
        } else {
            win->Maximize();
        }
        EmitWindowState(win);
        res.Success(ipc::Null());
    });

    on("window.close", [](const ipc::Request&, ipc::Response res) {
        auto win = window::GetActiveWindow();
        if (win) win->Close();
        res.Success(ipc::Null());
    });
}

}  // namespace desktop::features
