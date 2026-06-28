#include "components/button.h"

#include <utility>

#include <ftxui/component/component.hpp>
#include <ftxui/component/component_options.hpp>
#include <ftxui/dom/elements.hpp>

#include "theme/theme.h"

namespace hestia::tui {
    ftxui::Component pill_button(ftxui::ConstStringRef label, std::function<void()> on_click,
                                 const Theme &theme) {
        using namespace ftxui;

        const Decorator focused = theme.selected; // inverted pill (terminal colors)
        const Decorator resting = theme.muted;    // dim when not focused

        ButtonOption option;
        option.transform = [focused, resting](const EntryState &s) {
            Element e = text("  " + s.label + "  ");
            return s.focused ? e | focused : e | resting;
        };
        return Button(label, std::move(on_click), option);
    }
}
