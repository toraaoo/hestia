#include "core/browser/client.h"
#include "core/browser/client_manager.h"
#include "core/ipc/ipc_router.h"

#include "include/cef_command_ids.h"
#include "include/views/cef_browser_view.h"
#include "include/views/cef_window.h"

namespace desktop::browser {

    bool Client::OnProcessMessageReceived(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
                                          CefProcessId source_process, CefRefPtr<CefProcessMessage> message) {
        return ipc::GetBrowserRouter()->OnProcessMessageReceived(browser, frame, source_process, message);
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

    bool Client::OnBeforePopup(CefRefPtr<CefBrowser> /*browser*/, CefRefPtr<CefFrame> /*frame*/, int /*popup_id*/,
                               const CefString & /*target_url*/, const CefString & /*target_frame_name*/,
                               WindowOpenDisposition /*target_disposition*/, bool /*user_gesture*/,
                               const CefPopupFeatures & /*popupFeatures*/, CefWindowInfo & /*windowInfo*/,
                               CefRefPtr<CefClient> & /*client*/, CefBrowserSettings & /*settings*/,
                               CefRefPtr<CefDictionaryValue> & /*extra_info*/, bool * /*no_javascript_access*/) {
        return true;
    }

    void Client::OnTitleChange(CefRefPtr<CefBrowser> browser, const CefString &title) {
        PlatformTitleChange(browser, title);
    }

    void Client::OnDraggableRegionsChanged(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> /*frame*/,
                                           const std::vector<CefDraggableRegion> &regions) {
        if (auto view = CefBrowserView::GetForBrowser(browser)) {
            if (CefRefPtr<CefWindow> window = view->GetWindow()) {
                window->SetDraggableRegions(regions);
            }
        }
    }

    bool Client::OnDragEnter(CefRefPtr<CefBrowser> /*browser*/, CefRefPtr<CefDragData> /*dragData*/,
                             DragOperationsMask /*mask*/) {
        return true;
    }

    void Client::OnBeforeContextMenu(CefRefPtr<CefBrowser> /*browser*/, CefRefPtr<CefFrame> /*frame*/,
                                     CefRefPtr<CefContextMenuParams> /*params*/, CefRefPtr<CefMenuModel> model) {
        model->Clear();
    }

    bool Client::OnChromeCommand(CefRefPtr<CefBrowser> /*browser*/, int command_id,
                                 cef_window_open_disposition_t /*disposition*/) {
#if !defined(NDEBUG)
        switch (command_id) {
        case IDC_RELOAD:
        case IDC_RELOAD_BYPASSING_CACHE:
        case IDC_DEV_TOOLS:
        case IDC_DEV_TOOLS_CONSOLE:
        case IDC_DEV_TOOLS_DEVICES:
        case IDC_DEV_TOOLS_INSPECT:
        case IDC_DEV_TOOLS_TOGGLE: return false;
        default: break;
        }
#else
        (void)command_id;
#endif
        return true;
    }

} // namespace desktop::browser
