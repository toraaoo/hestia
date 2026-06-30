#pragma once

#include <string>

#include <hestia/client/client.h>

#include "navigation/view.h"

namespace hestia::tui {
    // Overview: the daemon's identity plus an interactive greeting, over the SDK.
    class HomeView : public View {
    public:
        RouteId id() const override;
        std::string title() const override;
        ftxui::Component build(AppContext &ctx) override;

    private:
        void load(AppContext &ctx);

        bool connected_ = false;
        std::string error_;
        client::AppInfo info_;

        std::string name_;
        std::string greeting_;
        std::string greet_error_;
    };
}
