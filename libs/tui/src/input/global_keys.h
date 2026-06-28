#pragma once

#include <ftxui/component/component_base.hpp>

namespace hestia::tui {
    struct AppContext;

    // Wrap a component so global keys are handled before they reach it. When an
    // overlay is open the wrapper steps aside (the modal handles its own keys),
    // keeping the binding modal-aware in one place.
    ftxui::Component with_global_keys(ftxui::Component base, AppContext &ctx);
}
