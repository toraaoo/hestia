#include "views/settings_view.h"

#include <exception>
#include <utility>

#include <ftxui/component/component.hpp>
#include <ftxui/dom/elements.hpp>
#include <spdlog/spdlog.h>

#include <hestia/client/client.h>

#include "app_context.h"
#include "components/button.h"
#include "components/panel.h"
#include "theme/theme.h"

namespace hestia::tui {
    RouteId SettingsView::id() const {
        return "settings";
    }

    std::string SettingsView::title() const {
        return "Settings";
    }

    void SettingsView::load(AppContext &ctx) {
        if (!ctx.client) return;
        try {
            autostart_enabled_ = ctx.client->autostart_status();
            autostart_known_ = true;
        } catch (const std::exception &e) {
            autostart_error_ = e.what();
            spdlog::warn("tui: settings could not read autostart status: {}", e.what());
        }
    }

    ftxui::Component SettingsView::build(AppContext &ctx) {
        using namespace ftxui;
        load(ctx);

        auto key_input = Input(&key_, "key");
        auto value_input = Input(&value_, "value");

        auto get_button = pill_button(
            "Get",
            [this, &ctx] {
                if (!ctx.client) return;
                try {
                    value_ = ctx.client->config_get(key_).value_or("");
                    config_status_ = value_.empty() ? "key not set" : "";
                } catch (const std::exception &e) {
                    config_status_ = e.what();
                }
            },
            *ctx.theme);

        auto set_button = pill_button(
            "Set",
            [this, &ctx] {
                if (!ctx.client) return;
                try {
                    ctx.client->config_set(key_, value_);
                    config_status_ = "saved";
                } catch (const std::exception &e) {
                    config_status_ = e.what();
                }
            },
            *ctx.theme);

        auto toggle_button = pill_button(
            "Toggle autostart",
            [this, &ctx] {
                if (!ctx.client) return;
                try {
                    if (autostart_enabled_)
                        ctx.client->autostart_disable();
                    else
                        ctx.client->autostart_enable();
                    autostart_enabled_ = !autostart_enabled_;
                    autostart_known_ = true;
                    autostart_error_.clear();
                } catch (const std::exception &e) {
                    autostart_error_ = e.what();
                }
            },
            *ctx.theme);

        auto container = Container::Vertical(
            {key_input, value_input, get_button, set_button, toggle_button});

        return Renderer(container, [this, &ctx, key_input, value_input, get_button,
                                    set_button, toggle_button] {
            const Theme &theme = *ctx.theme;

            Elements rows;
            if (!ctx.client) {
                rows.push_back(text("daemon unavailable") | theme.muted);
            } else {
                rows.push_back(text("config") | theme.emphasis);
                rows.push_back(hbox({text("key   ") | theme.muted,
                                     key_input->Render() | size(WIDTH, EQUAL, 24) | border}));
                rows.push_back(hbox({text("value ") | theme.muted,
                                     value_input->Render() | size(WIDTH, EQUAL, 24) | border}));
                rows.push_back(hbox({get_button->Render(), text("  "), set_button->Render()}));
                if (!config_status_.empty()) rows.push_back(text(config_status_) | theme.muted);

                rows.push_back(text(""));
                rows.push_back(text("autostart") | theme.emphasis);
                const std::string state =
                    autostart_known_ ? (autostart_enabled_ ? "enabled" : "disabled") : "unknown";
                rows.push_back(hbox({text("login start: ") | theme.muted, text(state) | theme.normal}));
                rows.push_back(toggle_button->Render());
                if (!autostart_error_.empty()) rows.push_back(text(autostart_error_) | theme.muted);
            }
            rows.push_back(filler());

            return panel("Settings", vbox(std::move(rows)), theme) | flex;
        });
    }
}
