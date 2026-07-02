#pragma once
#include "include/cef_browser.h"
#include <map>

namespace desktop::browser {

    class ClientManager {
    public:
        static ClientManager &Instance();

        void OnAfterCreated(CefRefPtr<CefBrowser> browser);
        void OnBeforeClose(CefRefPtr<CefBrowser> browser);

        // Returns any open browser (first in the map), or nullptr if none.
        CefRefPtr<CefBrowser> GetMainBrowser() const;

    private:
        std::map<int, CefRefPtr<CefBrowser>> browsers_;
    };

} // namespace desktop::browser
