#pragma once

#include <string>
#include <string_view>

namespace hestia::greeting {
    // Build a friendly greeting for `name`. Falls back to a generic greeting
    // when `name` is empty.
    std::string greet(std::string_view name = {});
}
