#pragma once

#include <string>

#include <ftxui/dom/elements.hpp>

namespace hestia::tui {
    struct AppContext;

    // Build the header slot: brand + active view title. Pure Element builder
    // (rebuilt every frame), styled from ctx.theme.
    ftxui::Element header_bar(const AppContext &ctx, const std::string &view_title);
}
