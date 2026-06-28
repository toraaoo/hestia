#include "layout/sidebar.h"

#include <string>

#include <ftxui/component/component.hpp>
#include <ftxui/component/component_options.hpp>
#include <ftxui/dom/elements.hpp>

#include "theme/theme.h"

namespace hestia::tui {
    ftxui::Component make_sidebar(const std::vector<std::string> *titles, int *selected,
                                  const Theme &theme) {
        using namespace ftxui;

        const Decorator active = theme.emphasis; // selected route: bold + arrow
        const Decorator focus = theme.selected;  // focused row: inverted highlight

        MenuOption option = MenuOption::Vertical();
        option.entries_option.transform = [active, focus](const EntryState &s) {
            Element e = text((s.active ? "▸ " : "  ") + s.label);
            if (s.active) e = e | active;
            if (s.focused) e = e | focus;
            return e;
        };
        return Menu(titles, selected, option);
    }
}
