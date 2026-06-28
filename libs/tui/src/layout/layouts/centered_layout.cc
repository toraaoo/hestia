#include "layout/layouts/centered_layout.h"

namespace hestia::tui {
    ftxui::Element CenteredLayout::arrange(const LayoutSlots &slots) const {
        using namespace ftxui;

        auto base = vbox({
            slots.header,
            separator(),
            slots.content | center | flex,
            separator(),
            slots.status,
        });

        return apply_overlay(base, slots.overlay);
    }
}
