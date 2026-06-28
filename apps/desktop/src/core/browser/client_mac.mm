#if defined(__APPLE__)
#include "core/browser/client.h"
#include "include/cef_browser.h"

namespace desktop::browser {

void PlatformTitleChange(CefRefPtr<CefBrowser> browser, const CefString& title) {
    (void)browser;
    (void)title;
}

}  // namespace desktop::browser
#endif
