#include "views/about_view.h"

#include <ftxui/component/component.hpp>
#include <ftxui/dom/elements.hpp>

#include "app_context.h"
#include "components/panel.h"
#include "theme/theme.h"

namespace hestia::tui {
    RouteId AboutView::id() const {
        return "about";
    }

    std::string AboutView::title() const {
        return "About";
    }

    LayoutId AboutView::layout() const {
        return layout::Centered; // one-line layout swap (vs the default Sidebar)
    }

    ftxui::Component AboutView::build(AppContext &ctx) {
        using namespace ftxui;

        // A view with no interactive widgets still needs a component to host its
        // renderer; an empty container suffices.
        auto container = Container::Vertical({});

        return Renderer(container, [&ctx] {
            const Theme &theme = *ctx.theme;
            auto body = vbox({
                text("Hestia") | theme.brand | hcenter,
                text(""),
                text("A personal home-management tool.") | theme.normal | hcenter,
                text("This view uses the Centered layout.") | theme.muted | hcenter,
            });
            return panel("About", body, theme) | size(WIDTH, GREATER_THAN, 40);
        });
    }
}
