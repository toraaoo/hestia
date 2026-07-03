#pragma once

#include <string>
#include <string_view>

namespace hestia::engine {
    // Falls back to a generic greeting when `name` is empty.
    std::string greet(std::string_view name = {});
} // namespace hestia::engine
