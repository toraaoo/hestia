#pragma once
#include "include/views/cef_window.h"

namespace desktop::window {

// The single active top-level window. Set by WindowDelegate::OnWindowCreated.
void SetActiveWindow(CefRefPtr<CefWindow> win);
CefRefPtr<CefWindow> GetActiveWindow();

// Minimize state is tracked manually since CefWindow has no IsMinimized().
void SetMinimized(bool minimized);
bool IsMinimized();

}  // namespace desktop::window
