#pragma once

namespace hestia::tui {
    // Launch the interactive terminal UI and run its event loop. Returns the
    // process exit code.
    //
    // This is the library's ONLY public symbol — the single seam between the CLI
    // app and the TUI. Everything else under libs/tui/src is private.
    int run();
}
