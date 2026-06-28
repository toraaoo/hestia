#pragma once

#include <string>
#include <utility>
#include <vector>

#include <ftxui/dom/elements.hpp>

namespace hestia::tui {
    struct Theme;

    // A single "key: action" hint, e.g.  q quit. Presentational only.
    ftxui::Element key_hint(const std::string &key, const std::string &label,
                            const Theme &theme);

    // A spaced row of hints for a status bar.
    ftxui::Element key_hints(const std::vector<std::pair<std::string, std::string>> &hints,
                             const Theme &theme);
}
