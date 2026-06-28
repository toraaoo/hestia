#pragma once
#include "core/app/app_base.h"
#include "include/cef_browser_process_handler.h"

namespace desktop::app {

class BrowserApp : public AppBase, public CefBrowserProcessHandler {
public:
    CefRefPtr<CefBrowserProcessHandler> GetBrowserProcessHandler() override { return this; }
    void OnContextInitialized() override;

private:
    IMPLEMENT_REFCOUNTING(BrowserApp);
};

}  // namespace desktop::app
