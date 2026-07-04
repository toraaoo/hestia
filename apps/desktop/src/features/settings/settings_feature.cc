#include "features/settings/settings_feature.h"

#include "core/daemon/daemon_client.h"
#include "core/ipc/ipc_router.h"

namespace desktop::features {

    void SettingsFeature::RegisterActions(ipc::Actions &on) {
        RegisterForward(on, "config.get", "config.get");
        RegisterForward(on, "config.set", "config.set");
    }

} // namespace desktop::features
