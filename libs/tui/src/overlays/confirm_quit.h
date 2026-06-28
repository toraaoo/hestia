#pragma once

#include <ftxui/component/component_base.hpp>

#include "navigation/navigator.h"

namespace hestia::tui {
    struct AppContext;

    namespace overlay {
        // Overlay id for the confirm-quit modal.
        inline const OverlayId ConfirmQuit = "confirm_quit";
    }

    // Build the confirm-quit modal: "Quit Hestia?" with Yes/No. Yes exits the
    // app; No (or Esc) closes the overlay. Built once and owned by the shell.
    ftxui::Component make_confirm_quit(AppContext &ctx);
}
