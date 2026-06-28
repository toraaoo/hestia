#pragma once

#include <map>
#include <memory>

#include "layout/layout.h"

namespace hestia::tui {
    // id -> Layout. Lookup never fails silently: an unknown id falls back to the
    // sidebar layout and logs a warning rather than crashing.
    class LayoutRegistry {
    public:
        void add(const LayoutId &id, std::unique_ptr<Layout> layout);
        const Layout &get(const LayoutId &id) const;

    private:
        std::map<LayoutId, std::unique_ptr<Layout>> layouts_;
    };

    // Build the registry of built-in layouts. Adding a layout = one line here.
    LayoutRegistry make_layouts();
}
