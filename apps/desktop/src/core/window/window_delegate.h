#pragma once
#include "include/views/cef_browser_view.h"
#include "include/views/cef_browser_view_delegate.h"
#include "include/views/cef_window.h"
#include "include/views/cef_window_delegate.h"

namespace desktop::window {

    // Hosts the BrowserView in a frameless top-level window.
    class WindowDelegate : public CefWindowDelegate {
    public:
        explicit WindowDelegate(CefRefPtr<CefBrowserView> view);

        void OnWindowCreated(CefRefPtr<CefWindow> window) override;
        void OnWindowDestroyed(CefRefPtr<CefWindow> window) override;
        bool IsFrameless(CefRefPtr<CefWindow>) override { return true; }
        bool CanResize(CefRefPtr<CefWindow>) override { return true; }
        bool CanClose(CefRefPtr<CefWindow>) override { return true; }
        CefSize GetMinimumSize(CefRefPtr<CefView>) override { return {800, 600}; }

    private:
        CefRefPtr<CefBrowserView> view_;
        IMPLEMENT_REFCOUNTING(WindowDelegate);
    };

    // Minimal BrowserView delegate — required by CefBrowserView::CreateBrowserView.
    class BrowserViewDelegate : public CefBrowserViewDelegate {
    public:
        void OnBrowserCreated(CefRefPtr<CefBrowserView>, CefRefPtr<CefBrowser>) override {}
        void OnBrowserDestroyed(CefRefPtr<CefBrowserView>, CefRefPtr<CefBrowser>) override {}

    private:
        IMPLEMENT_REFCOUNTING(BrowserViewDelegate);
    };

} // namespace desktop::window
