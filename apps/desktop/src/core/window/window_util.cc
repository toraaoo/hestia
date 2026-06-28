#include "core/window/window_util.h"

namespace desktop::window {

namespace {
CefRefPtr<CefWindow> g_active_window;
bool                 g_minimized = false;
}

void SetActiveWindow(CefRefPtr<CefWindow> win) { g_active_window = win; }
CefRefPtr<CefWindow> GetActiveWindow()          { return g_active_window; }

void SetMinimized(bool minimized) { g_minimized = minimized; }
bool IsMinimized()                { return g_minimized; }

}  // namespace desktop::window
