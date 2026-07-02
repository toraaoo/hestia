#include <hestia/engine/greeting/greeting.h>

#include <fmt/format.h>

namespace hestia::greeting {
    std::string greet(std::string_view name) {
        if (name.empty()) {
            return "Hello there!";
        }
        return fmt::format("Hello, {}!", name);
    }
} // namespace hestia::greeting
