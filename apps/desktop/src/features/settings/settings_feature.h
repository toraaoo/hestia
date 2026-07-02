#pragma once
#include "features/feature.h"

namespace desktop::features {

    class SettingsFeature : public Feature {
    public:
        const char *Name() const override { return "settings"; }
        void RegisterActions(ipc::Actions &on) override;
    };

} // namespace desktop::features
