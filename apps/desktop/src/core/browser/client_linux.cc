#include "core/browser/client.h"
#include "include/cef_browser.h"

namespace desktop::browser {

// Sets the X11/GTK window title to match the web-page title (for taskbars and
// window-switchers). The frameless window has no native title bar, but the WM
// still reads _NET_WM_NAME for the alt-tab label and task-switcher label.
void PlatformTitleChange(CefRefPtr<CefBrowser> browser, const CefString& title) {
    // CefBrowserView hosts the browser inside a CefWindow whose platform
    // handle can be retrieved. For now just let the WM pick a default.
    (void)browser;
    (void)title;
}

}  // namespace desktop::browser
