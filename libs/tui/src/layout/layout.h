#pragma once

#include <optional>
#include <string>

#include <ftxui/dom/elements.hpp>

namespace hestia::tui {
    // Open registry of layout ids, like route ids. A view names its layout by id.
    using LayoutId = std::string;

    namespace layout {
        inline const LayoutId Sidebar = "sidebar";       // default shell
        inline const LayoutId Fullscreen = "fullscreen"; // content only, no chrome
        inline const LayoutId Centered = "centered";     // boxed & centered
    }

    // The standard slots a layout arranges. These are Elements (per-frame render
    // output), NOT components: interactive components stay built-once in the
    // shell, which keeps focus/event routing centralised and layouts pure.
    struct LayoutSlots {
        ftxui::Element content; // the active view
        ftxui::Element header;
        ftxui::Element sidebar;
        ftxui::Element status;
        std::optional<ftxui::Element> overlay; // modal, when present
    };

    // A layout is a pure arranger: given the slots, it decides placement only.
    // Swapping layouts never rebuilds the component tree.
    class Layout {
    public:
        virtual ~Layout() = default;
        virtual ftxui::Element arrange(const LayoutSlots &) const = 0;
    };

    // Shared helper: stack a modal overlay above a (dimmed) base layer. Centred
    // by the overlay's own size. Used by every layout so overlay placement is
    // consistent regardless of which layout is active.
    inline ftxui::Element apply_overlay(ftxui::Element base,
                                        const std::optional<ftxui::Element> &overlay) {
        using namespace ftxui;
        if (!overlay) return base;
        return dbox({base | dim, *overlay | center});
    }
}
