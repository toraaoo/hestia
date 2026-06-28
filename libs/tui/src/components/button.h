#pragma once

#include <functional>

#include <ftxui/component/component_base.hpp>
#include <ftxui/util/ref.hpp>

namespace hestia::tui {
    struct Theme;

    // A low-chrome "pill" button: a padded label with no brackets or border. The
    // focused button is shown as an inverted pill (using the terminal's own
    // colors); unfocused buttons are dimmed. Presentational factory — the click
    // handler is supplied by the caller.
    ftxui::Component pill_button(ftxui::ConstStringRef label, std::function<void()> on_click,
                                 const Theme &theme);
}
