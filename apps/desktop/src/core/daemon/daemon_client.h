#pragma once

#include <string>

#include "core/ipc/ipc_router.h"

namespace desktop {

    void ForwardToDaemon(std::string channel, std::string payload_json, ipc::Response res);

    void RegisterForward(ipc::Actions &on, const std::string &action, const std::string &daemon_channel);

} // namespace desktop
