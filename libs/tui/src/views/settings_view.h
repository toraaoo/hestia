#pragma once

#include <string>

#include "navigation/view.h"

namespace hestia::tui {
    // Settings: config get/set and login autostart, over the client SDK — the
    // same operations the CLI and desktop expose.
    class SettingsView : public View {
    public:
        RouteId id() const override;
        std::string title() const override;
        ftxui::Component build(AppContext &ctx) override;

    private:
        void load(AppContext &ctx);

        std::string key_;
        std::string value_;
        std::string config_status_;

        bool autostart_enabled_ = false;
        bool autostart_known_ = false;
        std::string autostart_error_;
    };
}
