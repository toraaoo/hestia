#pragma once

#include <memory>
#include <vector>

#include "navigation/view.h"

namespace hestia::tui {
    // Construct the ordered set of views shown in the TUI. The single place a new
    // view is wired into the interface.
    std::vector<std::unique_ptr<View>> make_views();
}
