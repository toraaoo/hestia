#pragma once

#include <ftxui/component/event.hpp>

namespace hestia::tui::keys {
    // Global key predicates. Centralised here so bindings are declared in one
    // place rather than scattered through event handlers.

    inline bool quit(const ftxui::Event &e) {
        return e == ftxui::Event::Character('q');
    }

    inline bool cancel(const ftxui::Event &e) {
        return e == ftxui::Event::Escape;
    }
}
