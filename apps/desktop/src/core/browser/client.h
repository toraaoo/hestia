#pragma once
#include "include/cef_client.h"
#include "include/cef_command_handler.h"
#include "include/cef_context_menu_handler.h"
#include "include/cef_display_handler.h"
#include "include/cef_drag_handler.h"
#include "include/cef_life_span_handler.h"

namespace desktop::browser {

    class Client : public CefClient,
                   public CefLifeSpanHandler,
                   public CefDisplayHandler,
                   public CefDragHandler,
                   public CefContextMenuHandler,
                   public CefCommandHandler {
    public:
        // CefClient
        CefRefPtr<CefLifeSpanHandler> GetLifeSpanHandler() override { return this; }
        CefRefPtr<CefDisplayHandler> GetDisplayHandler() override { return this; }
        CefRefPtr<CefDragHandler> GetDragHandler() override { return this; }
        CefRefPtr<CefContextMenuHandler> GetContextMenuHandler() override { return this; }
        CefRefPtr<CefCommandHandler> GetCommandHandler() override { return this; }
        bool OnProcessMessageReceived(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
                                      CefProcessId source_process, CefRefPtr<CefProcessMessage> message) override;

        // CefLifeSpanHandler
        void OnAfterCreated(CefRefPtr<CefBrowser> browser) override;
        bool DoClose(CefRefPtr<CefBrowser> browser) override;
        void OnBeforeClose(CefRefPtr<CefBrowser> browser) override;
        bool OnBeforePopup(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame, int popup_id,
                           const CefString &target_url, const CefString &target_frame_name,
                           WindowOpenDisposition target_disposition, bool user_gesture,
                           const CefPopupFeatures &popupFeatures, CefWindowInfo &windowInfo,
                           CefRefPtr<CefClient> &client, CefBrowserSettings &settings,
                           CefRefPtr<CefDictionaryValue> &extra_info, bool *no_javascript_access) override;

        // CefDisplayHandler
        void OnTitleChange(CefRefPtr<CefBrowser> browser, const CefString &title) override;

        // CefDragHandler — forwards the page's -webkit-app-region regions to the
        // frameless window so marked elements (title/status bar) can drag the window.
        void OnDraggableRegionsChanged(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
                                       const std::vector<CefDraggableRegion> &regions) override;
        bool OnDragEnter(CefRefPtr<CefBrowser> browser, CefRefPtr<CefDragData> dragData,
                         DragOperationsMask mask) override;

        // The shell is an app window, not a browser: suppress the context menu,
        // Chrome accelerators, popup windows, and drag-in navigation. Debug builds
        // keep the DevTools and reload shortcuts for development.
        void OnBeforeContextMenu(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
                                 CefRefPtr<CefContextMenuParams> params, CefRefPtr<CefMenuModel> model) override;
        bool OnChromeCommand(CefRefPtr<CefBrowser> browser, int command_id,
                             cef_window_open_disposition_t disposition) override;

    private:
        IMPLEMENT_REFCOUNTING(Client);
    };

    // Implemented per-platform (client_linux.cc / client_win.cc / client_mac.mm).
    void PlatformTitleChange(CefRefPtr<CefBrowser> browser, const CefString &title);

} // namespace desktop::browser
