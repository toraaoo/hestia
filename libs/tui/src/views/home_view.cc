#include "views/home_view.h"

#include <exception>
#include <utility>

#include <ftxui/component/component.hpp>
#include <ftxui/dom/elements.hpp>
#include <spdlog/spdlog.h>

#include "app_context.h"
#include "components/button.h"
#include "components/panel.h"
#include "theme/theme.h"

namespace hestia::tui {
    RouteId HomeView::id() const {
        return "home";
    }

    std::string HomeView::title() const {
        return "Overview";
    }

    void HomeView::load(AppContext &ctx) {
        if (!ctx.client) return;
        try {
            info_ = ctx.client->app_info();
            connected_ = true;
        } catch (const std::exception &e) {
            error_ = e.what();
            spdlog::warn("tui: overview could not read app info: {}", e.what());
        }
    }

    ftxui::Component HomeView::build(AppContext &ctx) {
        using namespace ftxui;
        load(ctx);

        auto name_input = Input(&name_, "name");
        auto greet_button = pill_button(
            "Greet",
            [this, &ctx] {
                if (!ctx.client) {
                    greet_error_ = "daemon unavailable";
                    return;
                }
                try {
                    greeting_ = ctx.client->greet(name_);
                    greet_error_.clear();
                } catch (const std::exception &e) {
                    greet_error_ = e.what();
                }
            },
            *ctx.theme);
        auto quit_button = pill_button("Quit", ctx.request_quit, *ctx.theme);

        auto container = Container::Vertical({name_input, greet_button, quit_button});

        return Renderer(container, [this, &ctx, name_input, greet_button, quit_button] {
            const Theme &theme = *ctx.theme;

            auto field = [&](const std::string &label, const std::string &value) {
                return hbox({
                    text(label) | theme.muted | size(WIDTH, EQUAL, 10),
                    text(value) | theme.normal,
                });
            };

            Elements rows;
            if (connected_) {
                rows.push_back(field("name", info_.name));
                rows.push_back(field("version", info_.version));
                rows.push_back(field("channel", info_.channel));
                rows.push_back(field("vendor", info_.vendor));
            } else if (!ctx.client) {
                rows.push_back(text("daemon unavailable — start it with: hestiad serve") |
                               theme.muted);
            } else {
                rows.push_back(text("daemon error: " + error_) | theme.muted);
            }

            rows.push_back(text(""));
            rows.push_back(hbox({
                text("greet ") | theme.muted,
                name_input->Render() | size(WIDTH, EQUAL, 20) | border,
                text(" "),
                greet_button->Render(),
            }));
            if (!greeting_.empty()) rows.push_back(text(greeting_) | theme.emphasis);
            if (!greet_error_.empty()) rows.push_back(text(greet_error_) | theme.muted);

            rows.push_back(filler());
            rows.push_back(quit_button->Render() | hcenter);

            return panel("Overview", vbox(std::move(rows)), theme) | flex;
        });
    }
}
