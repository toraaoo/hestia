#include "core/window/window_util.h"

#include <cmrc/cmrc.hpp>

#include "include/cef_image.h"

CMRC_DECLARE(hestia_icons);

namespace desktop::window {

    namespace {
        CefRefPtr<CefWindow> g_active_window;
        bool g_minimized = false;

        CefRefPtr<CefImage> MakeIcon(const char *png_1x, const char *png_2x) {
            auto fs = cmrc::hestia_icons::get_filesystem();
            auto image = CefImage::CreateImage();
            const auto add = [&](float scale, const char *path) {
                cmrc::file f = fs.open(path);
                image->AddPNG(scale, f.begin(), f.size());
            };
            add(1.0f, png_1x);
            add(2.0f, png_2x);
            return image;
        }
    } // namespace

    void SetActiveWindow(CefRefPtr<CefWindow> win) {
        g_active_window = win;
    }
    CefRefPtr<CefWindow> GetActiveWindow() {
        return g_active_window;
    }

    void SetMinimized(bool minimized) {
        g_minimized = minimized;
    }
    bool IsMinimized() {
        return g_minimized;
    }

    void ApplyWindowIcons(CefRefPtr<CefWindow> win) {
        win->SetWindowIcon(MakeIcon("hestia-16.png", "hestia-32.png"));
        win->SetWindowAppIcon(MakeIcon("hestia-128.png", "hestia-256.png"));
    }

} // namespace desktop::window
