#include "input/global_keys.h"

#include <ftxui/component/component.hpp>

#include "app_context.h"
#include "input/keymap.h"
#include "navigation/navigator.h"

namespace hestia::tui {
    ftxui::Component with_global_keys(ftxui::Component base, AppContext &ctx) {
        using namespace ftxui;
        return CatchEvent(std::move(base), [&ctx](const Event &e) {
            // Modal: let the overlay own all input while it is open.
            if (ctx.nav && ctx.nav->has_overlay()) return false;
            if (keys::quit(e) || keys::cancel(e)) {
                ctx.request_quit();
                return true;
            }
            return false;
        });
    }
}
