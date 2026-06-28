#include "layout/layouts/fullscreen_layout.h"

namespace hestia::tui {
    ftxui::Element FullscreenLayout::arrange(const LayoutSlots &slots) const {
        using namespace ftxui;
        return apply_overlay(slots.content | flex, slots.overlay);
    }
}
