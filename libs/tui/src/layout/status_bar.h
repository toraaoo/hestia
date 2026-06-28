#pragma once

#include <ftxui/dom/elements.hpp>

namespace hestia::tui {
    struct AppContext;

    // Build the status slot: global key hints. Pure Element builder, styled from
    // ctx.theme.
    ftxui::Element status_bar(const AppContext &ctx);
}
