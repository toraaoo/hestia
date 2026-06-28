#include "layout/layouts/sidebar_layout.h"

namespace hestia::tui {
    ftxui::Element SidebarLayout::arrange(const LayoutSlots &slots) const {
        using namespace ftxui;

        // Each region floats in its own rounded panel. The content slot already
        // arrives wrapped in its view's panel, so the layout frames only the
        // header and the nav rail, then leaves a one-column gutter before content.
        auto header_box = slots.header | borderRounded;

        auto sidebar_box = slots.sidebar | borderRounded | size(WIDTH, EQUAL, 20) | yflex;

        auto body = hbox({
                        sidebar_box,
                        text(" "), // gutter between the rail and the content panel
                        slots.content | flex,
                    }) |
                    yflex;

        auto base = vbox({
            header_box,
            body,
            slots.status,
        });

        return apply_overlay(base, slots.overlay);
    }
}
