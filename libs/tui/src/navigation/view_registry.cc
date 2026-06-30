#include "navigation/view_registry.h"

#include "views/home_view.h"
#include "views/settings_view.h"

namespace hestia::tui {
    std::vector<std::unique_ptr<View>> make_views() {
        std::vector<std::unique_ptr<View>> views;
        views.push_back(std::make_unique<HomeView>());
        views.push_back(std::make_unique<SettingsView>());
        return views;
    }
}
