#pragma once

#include <cstdint>
#include <filesystem>
#include <functional>

namespace hestia::engine {
    using ExtractProgressCallback = std::function<void(std::uint64_t done, std::uint64_t total)>;

    // Spawns the system tar: every supported platform ships one that reads the
    // archive it is given (.tar.gz on Linux/macOS, .zip via Windows 10+ bsdtar).
    // Progress is per archive entry — a listing pass counts them, then the
    // extraction pass reports each one.
    void extract_archive(const std::filesystem::path &archive, const std::filesystem::path &dest,
                         const ExtractProgressCallback &on_progress = {});
} // namespace hestia::engine
