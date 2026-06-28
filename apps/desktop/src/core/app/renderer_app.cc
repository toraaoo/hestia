#include "core/app/renderer_app.h"
#include "include/cef_process_message.h"

namespace desktop::app {

namespace {
// Shared router config — must match the browser-side config in ipc_router.cc.
CefMessageRouterConfig RouterConfig() {
    CefMessageRouterConfig cfg;
    cfg.js_query_function  = "cefQuery";
    cfg.js_cancel_function = "cefQueryCancel";
    return cfg;
}
}  // namespace

void RendererApp::OnWebKitInitialized() {
    // Registers the native window.cefQuery / window.cefQueryCancel bindings.
    message_router_ = CefMessageRouterRendererSide::Create(RouterConfig());
}

void RendererApp::OnContextCreated(CefRefPtr<CefBrowser> browser,
                                   CefRefPtr<CefFrame> frame,
                                   CefRefPtr<CefV8Context> context) {
    message_router_->OnContextCreated(browser, frame, context);
}

void RendererApp::OnContextReleased(CefRefPtr<CefBrowser> browser,
                                    CefRefPtr<CefFrame> frame,
                                    CefRefPtr<CefV8Context> context) {
    if (message_router_)
        message_router_->OnContextReleased(browser, frame, context);
}

bool RendererApp::OnProcessMessageReceived(CefRefPtr<CefBrowser> browser,
                                           CefRefPtr<CefFrame> frame,
                                           CefProcessId source_process,
                                           CefRefPtr<CefProcessMessage> message) {
    // Native → JS events: dispatch as DOM CustomEvent so JS can subscribe via
    // window.addEventListener(channel, ...) (see frontend/src/lib/ipc.ts).
    if (message->GetName() == "hestia.emit") {
        auto args    = message->GetArgumentList();
        auto channel = args->GetString(0).ToString();
        auto payload = args->GetString(1).ToString();
        std::string js =
            "window.dispatchEvent(new CustomEvent('" + channel +
            "', { detail: " + payload + " }));";
        frame->ExecuteJavaScript(js, "", 0);
        return true;
    }
    if (message_router_)
        return message_router_->OnProcessMessageReceived(browser, frame, source_process, message);
    return false;
}

}  // namespace desktop::app
