#include "core/daemon/daemon_client.h"

#include <utility>

#include <hestia/client/bridge.h>

#include "include/base/cef_bind.h"
#include "include/base/cef_callback.h"
#include "include/cef_task.h"
#include "include/wrapper/cef_closure_task.h"

namespace desktop {
namespace {

void Deliver(std::string channel, std::string payload_json, ipc::Response res) {
    const auto reply = hestia::client::call_daemon(channel, payload_json);
    if (reply.ok)
        res.Success(reply.json);
    else
        res.Failure(1, reply.error);
}

}  // namespace

void ForwardToDaemon(std::string channel, std::string payload_json, ipc::Response res) {
    CefPostTask(TID_FILE_USER_BLOCKING,
                base::BindOnce(&Deliver, std::move(channel), std::move(payload_json),
                               std::move(res)));
}

void RegisterForward(ipc::Actions& on, const std::string& action,
                     const std::string& daemon_channel) {
    on(action, [daemon_channel](const ipc::Request& req, ipc::Response res) {
        ForwardToDaemon(daemon_channel, req.payload_raw, std::move(res));
    });
}

}  // namespace desktop
