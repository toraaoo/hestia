#include "features/feature_registry.h"
#include "features/feature.h"
#include "features/app/app_feature.h"
#include "features/settings/settings_feature.h"
#include "core/ipc/ipc_router.h"
#include <memory>
#include <vector>

namespace desktop::features {

static std::vector<std::unique_ptr<Feature>> BuildFeatures() {
    std::vector<std::unique_ptr<Feature>> f;
    f.push_back(std::make_unique<AppFeature>());
    f.push_back(std::make_unique<SettingsFeature>());
    return f;
}

void RegisterAll() {
    for (auto& feat : BuildFeatures()) {
        ipc::Actions on(feat->Name(), ipc::Registry::Instance());
        feat->RegisterActions(on);
    }
}

}  // namespace desktop::features
