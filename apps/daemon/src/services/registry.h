#pragma once

#include <memory>
#include <vector>

#include "services/service.h"

namespace hestia::daemon {
    std::vector<std::unique_ptr<Service>> make_services();
} // namespace hestia::daemon
