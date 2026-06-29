#include "core/browser/client.h"
#include "core/browser/client_manager.h"
#include "core/ipc/ipc_router.h"

#include "include/views/cef_browser_view.h"
#include "include/views/cef_window.h"

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

void Client::OnDraggableRegionsChanged(CefRefPtr<CefBrowser> browser,
                                       CefRefPtr<CefFrame> /*frame*/,
                                       const std::vector<CefDraggableRegion>& regions) {
    if (auto view = CefBrowserView::GetForBrowser(browser)) {
        if (CefRefPtr<CefWindow> window = view->GetWindow()) {
            window->SetDraggableRegions(regions);
        }
    }
}

}  // namespace desktop::browser
