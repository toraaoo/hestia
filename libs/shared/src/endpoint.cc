#include "hestia/ipc/endpoint.h"

#if !defined(_WIN32)
#include <cstdlib>
#include <string>
#include <unistd.h>

namespace hestia::ipc {
    namespace fs = std::filesystem;

    fs::path runtime_dir() {
        // Prefer the session runtime dir (tmpfs, auto-cleaned at logout). Fall
        // back to a uid-scoped /tmp dir when it is unset (e.g. non-login shells,
        // cron) so two users never collide on one socket path.
        if (const char *xdg = std::getenv("XDG_RUNTIME_DIR"); xdg && *xdg) {
            return fs::path(xdg) / "hestia";
        }
        return fs::path("/tmp") / ("hestia-" + std::to_string(::getuid()));
    }

    fs::path default_endpoint() {
        return runtime_dir() / "hestiad.sock";
    }
}
#else
// Windows named-pipe endpoint resolution lands with the Windows transport.
namespace hestia::ipc {
    std::filesystem::path runtime_dir() {
        return std::filesystem::temp_directory_path() / "hestia";
    }
    std::filesystem::path default_endpoint() {
        return R"(\\.\pipe\hestia-hestiad)";
    }
}
#endif
