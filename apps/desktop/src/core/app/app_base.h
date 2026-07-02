#pragma once
#include "include/cef_app.h"

namespace desktop::app {

    // Registers the custom URL scheme on every CEF process type (browser,
    // renderer, gpu, utility). Derived classes add their process-specific handlers.
    class AppBase : public CefApp {
    public:
        void OnRegisterCustomSchemes(CefRawPtr<CefSchemeRegistrar> registrar) override;
    };

} // namespace desktop::app
