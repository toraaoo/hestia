#pragma once

namespace hestia::cli {
    // Parse the command line and run the selected command. Returns the process
    // exit code.
    int run(int argc, char **argv);
}
