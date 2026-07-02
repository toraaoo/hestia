#pragma once
#include "features/feature.h"

namespace desktop::features {

    // Handles the "app.*" IPC channels:  app.info, app.ping
    class AppFeature : public Feature {
    public:
        const char *Name() const override { return "app"; }
        void RegisterActions(ipc::Actions &on) override;
    };

} // namespace desktop::features
