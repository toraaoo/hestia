#pragma once

#include <cstdint>
#include <memory>

#include "process/process_types.h"

// The LivenessProbe seam: "is this pid still ours?". Isolated so the OS-specific
// mechanism (Linux /proc + kill(0) today; pidfd/kqueue later) can be swapped or
// faked in tests. Answers liveness for processes re-adopted after a restart.
namespace hestia::daemon {
    class LivenessProbe {
    public:
        virtual ~LivenessProbe() = default;

        // Is a process with this pid currently alive?
        virtual bool is_alive(std::int64_t pid) const = 0;

        // An opaque, monotonic-per-process start time used to disambiguate PID
        // reuse. 0 means "unavailable on this platform".
        virtual std::int64_t read_start_time(std::int64_t pid) const = 0;

        // Require a verifiable start time on both sides: is_alive is true even for
        // another user's process (EPERM), so a bare-pid match could later SIGTERM a
        // stranger's process group after PID reuse.
        bool matches(const ProcessRecord &rec) const {
            if (!is_alive(rec.pid)) return false;
            const std::int64_t current = read_start_time(rec.pid);
            if (rec.start_time == 0 || current == 0) return false;
            return current == rec.start_time;
        }
    };

    // The platform liveness probe.
    std::unique_ptr<LivenessProbe> make_liveness_probe();
}
