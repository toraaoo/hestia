#include "components/panel.h"

#include "theme/theme.h"

namespace hestia::tui {
    ftxui::Element panel(const std::string &title, ftxui::Element body, const Theme &theme) {
        using namespace ftxui;
        return vbox({
                   text(" " + title + " ") | theme.brand,
                   text(""),
                   body | flex,
               }) |
               borderRounded;
    }
}
