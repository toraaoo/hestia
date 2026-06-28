#pragma once

#include <string>

#include <ftxui/dom/elements.hpp>

namespace hestia::tui {
    struct Theme;

    // A titled, bordered container around arbitrary content. Presentational only:
    // takes its body as a ready-made Element (props-in), holds no state or logic.
    ftxui::Element panel(const std::string &title, ftxui::Element body, const Theme &theme);
}
