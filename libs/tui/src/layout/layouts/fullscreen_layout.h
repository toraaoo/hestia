#pragma once

#include "layout/layout.h"

namespace hestia::tui {
    // Focus mode: the content fills the screen, no header/sidebar/status. For
    // immersive views where chrome would distract.
    class FullscreenLayout : public Layout {
    public:
        ftxui::Element arrange(const LayoutSlots &slots) const override;
    };
}
