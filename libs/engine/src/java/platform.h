#pragma once

#include <filesystem>
#include <optional>

namespace hestia::engine {
    // Handles the flat JDK layout and the macOS Contents/Home nesting.
    std::optional<std::filesystem::path> find_java_executable(const std::filesystem::path &root);
} // namespace hestia::engine
