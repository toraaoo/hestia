#include "core/browser/client.h"
#include "core/browser/client_manager.h"
#include "core/ipc/ipc_router.h"

namespace desktop::browser {

bool Client::OnProcessMessageReceived(CefRefPtr<CefBrowser> browser,
                                      CefRefPtr<CefFrame> frame,
                                      CefProcessId source_process,
                                      CefRefPtr<CefProcessMessage> message) {
    return ipc::GetBrowserRouter()->OnProcessMessageReceived(
        browser, frame, source_process, message);
}

void Client::OnAfterCreated(CefRefPtr<CefBrowser> browser) {
    ClientManager::Instance().OnAfterCreated(browser);
}

bool Client::DoClose(CefRefPtr<CefBrowser> browser) {
    return false;
}

void Client::OnBeforeClose(CefRefPtr<CefBrowser> browser) {
    ipc::GetBrowserRouter()->OnBeforeClose(browser);
    ClientManager::Instance().OnBeforeClose(browser);
}

void Client::OnTitleChange(CefRefPtr<CefBrowser> browser, const CefString& title) {
    PlatformTitleChange(browser, title);
}

}  // namespace desktop::browser
