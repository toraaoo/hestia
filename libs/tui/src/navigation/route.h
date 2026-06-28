#pragma once

#include <string>

namespace hestia::tui {
    // Stable identifier for a view/route, used by the Navigator to address views
    // by name (e.g. goto_route("home")). An open registry, like a URL path.
    using RouteId = std::string;
}
