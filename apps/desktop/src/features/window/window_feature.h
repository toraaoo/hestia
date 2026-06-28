#pragma once
#include "features/feature.h"

namespace desktop::features {

// Handles the "window.*" IPC channels:
//   window.minimize, window.maximize, window.close, window.state
class WindowFeature : public Feature {
public:
    const char* Name() const override { return "window"; }
    void RegisterActions(ipc::Actions& on) override;
};

}  // namespace desktop::features
