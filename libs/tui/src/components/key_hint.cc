#include "components/key_hint.h"

#include "theme/theme.h"

namespace hestia::tui {
    ftxui::Element key_hint(const std::string &key, const std::string &label,
                            const Theme &theme) {
        using namespace ftxui;
        return hbox({
            text(key) | theme.emphasis,
            text(" " + label) | theme.muted,
        });
    }

    ftxui::Element key_hints(const std::vector<std::pair<std::string, std::string>> &hints,
                             const Theme &theme) {
        using namespace ftxui;
        Elements row;
        for (std::size_t i = 0; i < hints.size(); ++i) {
            if (i) row.push_back(text("   "));
            row.push_back(key_hint(hints[i].first, hints[i].second, theme));
        }
        return hbox(std::move(row));
    }
}
