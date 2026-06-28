#pragma once

#include "navigation/view.h"

namespace hestia::tui {
    // Demo view that selects a non-default layout: it returns layout::Centered to
    // show how a single override swaps the arrangement (the sidebar disappears
    // and the panel centres) without any change to the component tree.
    class AboutView : public View {
    public:
        RouteId id() const override;
        std::string title() const override;
        LayoutId layout() const override;
        ftxui::Component build(AppContext &ctx) override;
    };
}
