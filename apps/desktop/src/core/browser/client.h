#pragma once
#include "include/cef_client.h"
#include "include/cef_display_handler.h"
#include "include/cef_life_span_handler.h"
#include "include/wrapper/cef_message_router.h"

namespace desktop::browser {

class Client : public CefClient,
               public CefLifeSpanHandler,
               public CefDisplayHandler {
public:
    // CefClient
    CefRefPtr<CefLifeSpanHandler> GetLifeSpanHandler() override { return this; }
    CefRefPtr<CefDisplayHandler>  GetDisplayHandler()  override { return this; }
    bool OnProcessMessageReceived(CefRefPtr<CefBrowser> browser,
                                  CefRefPtr<CefFrame> frame,
                                  CefProcessId source_process,
                                  CefRefPtr<CefProcessMessage> message) override;

    // CefLifeSpanHandler
    void OnAfterCreated(CefRefPtr<CefBrowser> browser) override;
    bool DoClose(CefRefPtr<CefBrowser> browser) override;
    void OnBeforeClose(CefRefPtr<CefBrowser> browser) override;

    // CefDisplayHandler
    void OnTitleChange(CefRefPtr<CefBrowser> browser, const CefString& title) override;

private:
    IMPLEMENT_REFCOUNTING(Client);
};

// Implemented per-platform (client_linux.cc / client_win.cc / client_mac.mm).
void PlatformTitleChange(CefRefPtr<CefBrowser> browser, const CefString& title);

}  // namespace desktop::browser
