#pragma once

#include <string>
#include <vector>

#include <ftxui/component/component_base.hpp>

namespace hestia::tui {
    struct Theme;

    // Build the navigation sidebar: an interactive Menu over the view titles,
    // bound to the shared selection index. This is a built-once *component*
    // (events/focus), unlike the header/status slots which are per-frame
    // Elements — the menu must retain focus state across frames.
    ftxui::Component make_sidebar(const std::vector<std::string> *titles, int *selected,
                                  const Theme &theme);
}
