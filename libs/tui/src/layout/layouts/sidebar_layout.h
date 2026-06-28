#pragma once

#include "layout/layout.h"

namespace hestia::tui {
    // Default shell: header on top, navigation sidebar beside the content, status
    // bar at the bottom. The everyday chrome.
    class SidebarLayout : public Layout {
    public:
        ftxui::Element arrange(const LayoutSlots &slots) const override;
    };
}
