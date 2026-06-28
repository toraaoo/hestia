#include "layout/layout_registry.h"

#include <utility>

#include <spdlog/spdlog.h>

#include "layout/layouts/centered_layout.h"
#include "layout/layouts/fullscreen_layout.h"
#include "layout/layouts/sidebar_layout.h"

namespace hestia::tui {
    void LayoutRegistry::add(const LayoutId &id, std::unique_ptr<Layout> layout) {
        layouts_[id] = std::move(layout);
    }

    const Layout &LayoutRegistry::get(const LayoutId &id) const {
        if (auto it = layouts_.find(id); it != layouts_.end())
            return *it->second;
        spdlog::warn("tui: unknown layout id '{}', falling back to '{}'", id, layout::Sidebar);
        // Sidebar is always registered by make_layouts(); this is safe.
        return *layouts_.at(layout::Sidebar);
    }

    LayoutRegistry make_layouts() {
        LayoutRegistry r;
        r.add(layout::Sidebar, std::make_unique<SidebarLayout>());
        r.add(layout::Fullscreen, std::make_unique<FullscreenLayout>());
        r.add(layout::Centered, std::make_unique<CenteredLayout>());
        return r;
    }
}
