#include "features/settings/settings_feature.h"

#include "core/daemon/daemon_client.h"
#include "core/ipc/ipc_router.h"

namespace desktop::features {

void SettingsFeature::RegisterActions(ipc::Actions& on) {
    RegisterForward(on, "config.get", "config.get");
    RegisterForward(on, "config.set", "config.set");
    RegisterForward(on, "config.home", "config.home");
    RegisterForward(on, "config.set-home", "config.set-home");
    RegisterForward(on, "autostart.status", "autostart.status");
    RegisterForward(on, "autostart.enable", "autostart.enable");
    RegisterForward(on, "autostart.disable", "autostart.disable");
}

}  // namespace desktop::features
