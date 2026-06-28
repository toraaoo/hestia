#pragma once

#include <ftxui/dom/elements.hpp>

namespace hestia::tui {
    // Visual styling for the TUI, expressed as semantic *roles* rather than fixed
    // colors. Each role is an ftxui::Decorator built only from terminal-honoring
    // primitives — Color::Default and text attributes (bold/dim/inverted). We
    // deliberately enforce NO palette of our own: hierarchy is conveyed through
    // attributes, so the UI inherits the user's terminal theme (foreground,
    // background, and the 16 ANSI colors they have configured).
    //
    // To restyle, change the decorators here; layouts and components pull their
    // styling from these roles and never hard-code a color.
    struct Theme {
        // Branding / primary headings.
        ftxui::Decorator brand = ftxui::bold;
        // Emphasised text (active titles, key captions).
        ftxui::Decorator emphasis = ftxui::bold;
        // De-emphasised text (hints, secondary labels).
        ftxui::Decorator muted = ftxui::dim;
        // Selected / highlighted row — swaps fg/bg using the terminal's own
        // colors rather than imposing ours.
        ftxui::Decorator selected = ftxui::inverted;
        // Plain body text: identity, i.e. the terminal default.
        ftxui::Decorator normal = ftxui::nothing;
    };
}
