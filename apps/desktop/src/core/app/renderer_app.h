#pragma once
#include "core/app/app_base.h"
#include "include/cef_render_process_handler.h"
#include "include/wrapper/cef_message_router.h"

namespace desktop::app {

class RendererApp : public AppBase, public CefRenderProcessHandler {
public:
    CefRefPtr<CefRenderProcessHandler> GetRenderProcessHandler() override { return this; }

    // The renderer-side router MUST be created here: this is when the message
    // router registers the native window.cefQuery binding with V8. Creating it
    // later (e.g. in OnContextCreated) misses that window and cefQuery is never
    // injected — the frontend then reports the bridge as "detached".
    void OnWebKitInitialized() override;

    void OnContextCreated(CefRefPtr<CefBrowser> browser,
                          CefRefPtr<CefFrame> frame,
                          CefRefPtr<CefV8Context> context) override;
    void OnContextReleased(CefRefPtr<CefBrowser> browser,
                           CefRefPtr<CefFrame> frame,
                           CefRefPtr<CefV8Context> context) override;
    bool OnProcessMessageReceived(CefRefPtr<CefBrowser> browser,
                                  CefRefPtr<CefFrame> frame,
                                  CefProcessId source_process,
                                  CefRefPtr<CefProcessMessage> message) override;

private:
    CefRefPtr<CefMessageRouterRendererSide> message_router_;
    IMPLEMENT_REFCOUNTING(RendererApp);
};

}  // namespace desktop::app
