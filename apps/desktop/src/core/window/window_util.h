#pragma once
#include "include/views/cef_window.h"

namespace desktop::window {

    // The single active top-level window. Set by WindowDelegate::OnWindowCreated.
    void SetActiveWindow(CefRefPtr<CefWindow> win);
    CefRefPtr<CefWindow> GetActiveWindow();

    // Minimize state is tracked manually since CefWindow has no IsMinimized().
    void SetMinimized(bool minimized);
    bool IsMinimized();

    // Window + taskbar/app-switcher icons from the embedded PNGs (WM_SETICON on
    // Windows, _NET_WM_ICON on X11; Wayland resolves icons via the .desktop file).
    void ApplyWindowIcons(CefRefPtr<CefWindow> win);

} // namespace desktop::window
