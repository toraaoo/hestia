#include "overlays/confirm_quit.h"

#include <ftxui/component/component.hpp>
#include <ftxui/component/event.hpp>
#include <ftxui/dom/elements.hpp>

#include "app_context.h"
#include "components/button.h"
#include "theme/theme.h"

namespace hestia::tui {
    ftxui::Component make_confirm_quit(AppContext &ctx) {
        using namespace ftxui;

        auto yes = pill_button("Yes", [&ctx] { ctx.exit_app(); }, *ctx.theme);
        auto no = pill_button("No", [&ctx] { ctx.nav->close_overlay(); }, *ctx.theme);

        auto buttons = Container::Horizontal({yes, no});

        auto body = Renderer(buttons, [yes, no, &ctx] {
            const Theme &theme = *ctx.theme;
            return vbox({
                       text("Quit Hestia?") | theme.emphasis | hcenter,
                       text(""),
                       hbox({
                           yes->Render(),
                           text("  "),
                           no->Render(),
                       }) | hcenter,
                   }) |
                   border | size(WIDTH, GREATER_THAN, 28);
        });

        // Esc cancels the modal, matching the global cancel binding.
        return CatchEvent(body, [&ctx](const Event &e) {
            if (e == Event::Escape) {
                ctx.nav->close_overlay();
                return true;
            }
            return false;
        });
    }
}
