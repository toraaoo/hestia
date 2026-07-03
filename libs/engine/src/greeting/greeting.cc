#include <hestia/engine/greeting.h>

#include <fmt/format.h>

namespace hestia::engine {
    std::string greet(std::string_view name) {
        if (name.empty()) {
            return "Hello there!";
        }
        return fmt::format("Hello, {}!", name);
    }
} // namespace hestia::engine
