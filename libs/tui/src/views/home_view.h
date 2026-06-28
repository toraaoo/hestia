#pragma once

#include "navigation/view.h"

namespace hestia::tui {
    // Landing view: a welcome panel with a quit affordance. Uses the default
    // sidebar layout. Establishes the one-file-per-view pattern.
    class HomeView : public View {
    public:
        RouteId id() const override;
        std::string title() const override;
        ftxui::Component build(AppContext &ctx) override;
    };
}
