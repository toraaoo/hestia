#include "core/browser/client_manager.h"
#include "include/cef_app.h"

namespace desktop::browser {

    ClientManager &ClientManager::Instance() {
        static ClientManager instance;
        return instance;
    }

    void ClientManager::OnAfterCreated(CefRefPtr<CefBrowser> browser) {
        browsers_[browser->GetIdentifier()] = browser;
    }

    void ClientManager::OnBeforeClose(CefRefPtr<CefBrowser> browser) {
        browsers_.erase(browser->GetIdentifier());
        if (browsers_.empty()) CefQuitMessageLoop();
    }

    CefRefPtr<CefBrowser> ClientManager::GetMainBrowser() const {
        if (browsers_.empty()) return nullptr;
        return browsers_.begin()->second;
    }

} // namespace desktop::browser
