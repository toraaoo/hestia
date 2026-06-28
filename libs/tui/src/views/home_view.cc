#include "views/home_view.h"

#include <ftxui/component/component.hpp>
#include <ftxui/dom/elements.hpp>

#include "app_context.h"
#include "components/button.h"
#include "components/panel.h"
#include "theme/theme.h"

namespace hestia::tui {
    RouteId HomeView::id() const {
        return "home";
    }

    std::string HomeView::title() const {
        return "Home";
    }

    ftxui::Component HomeView::build(AppContext &ctx) {
        using namespace ftxui;

        auto quit_button = pill_button("Quit", ctx.request_quit, *ctx.theme);

        auto container = Container::Vertical({quit_button});

        return Renderer(container, [quit_button, &ctx] {
            const Theme &theme = *ctx.theme;
            auto body = vbox({
                text("Welcome to the Hestia terminal UI.") | theme.normal,
                text(""),
                text("Pick a section from the sidebar.") | theme.muted,
                filler(),
                quit_button->Render() | hcenter,
            });
            return panel("Home", body, theme) | flex;
        });
    }
}
