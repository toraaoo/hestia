#pragma once

#include <hestia/proto/process.h>

// The process domain vocabulary comes from the shared wire contracts, so the
// daemon-internal code reads `ProcessRecord`, not `proto::ProcessRecord`.
namespace hestia::daemon {
    using proto::LaunchSpec;
    using proto::ProcessKind;
    using proto::ProcessRecord;
    using proto::ProcessState;
    using proto::RestartPolicy;
} // namespace hestia::daemon
