#if defined(_WIN32)
#include "core/browser/client.h"
#include "include/cef_browser.h"
#include <windows.h>

namespace desktop::browser {

void PlatformTitleChange(CefRefPtr<CefBrowser> browser, const CefString& title) {
    (void)browser;
    (void)title;
}

}  // namespace desktop::browser
#endif
