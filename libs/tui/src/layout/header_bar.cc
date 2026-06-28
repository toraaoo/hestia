#include "layout/header_bar.h"

#include "app_context.h"
#include "theme/theme.h"

namespace hestia::tui {
    ftxui::Element header_bar(const AppContext &ctx, const std::string &view_title) {
        using namespace ftxui;
        const Theme &theme = *ctx.theme;
        return hbox({
            text(" HESTIA") | theme.brand,
            filler(),
            text(view_title + " ") | theme.emphasis,
        });
    }
}
