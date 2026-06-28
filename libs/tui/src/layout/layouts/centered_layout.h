#pragma once

#include "layout/layout.h"

namespace hestia::tui {
    // Content boxed and centred, with header/status retained but no sidebar. Suits
    // wizards and dialog-like views that want the user's focus on one panel.
    class CenteredLayout : public Layout {
    public:
        ftxui::Element arrange(const LayoutSlots &slots) const override;
    };
}
