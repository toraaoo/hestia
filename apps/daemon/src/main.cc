#include "runtime/server.h"

#include <hestia/app_info.h>
#include <hestia/client/client.h>
#include <hestia/ipc/endpoint.h>
#include <hestia/logging.h>

#include <CLI/CLI.hpp>

#include <exception>
#include <iostream>
#include <string>

// hestiad — the Hestia daemon.
//
//   hestiad [serve]    run the daemon: bind the endpoint, serve until signalled
//   hestiad ping       connect to a running daemon, report its identity
//
// main() only does bootstrap: CLI parsing, logging init, and dispatch. The serve
// loop and the daemon's Runtime live in runtime/server.cc; every channel lives in
// a service under src/services/.
namespace {
    // `hestiad ping` reuses the client SDK's transport rather than reimplementing
    // connect/encode/recv: connecting performs the version handshake, so a clean
    // round-trip proves the daemon is reachable and compatible.
    int run_ping() {
        try {
            auto client = hestia::client::Client::connect(/*auto_spawn=*/false);
            const auto info = client.app_info();
            std::cout << info.name << ' ' << info.version << " — alive\n";
            return 0;
        } catch (const std::exception &e) {
            std::cerr << "hestiad ping: " << e.what() << '\n';
            return 1;
        }
    }
} // namespace

int main(int argc, char **argv) {
    CLI::App app{"hestiad — the Hestia daemon"};
    app.set_version_flag("--version", std::string(APP_NAME) + " " + APP_VERSION);
    app.fallthrough(); // accept the global -v/-q flags after the subcommand too

    bool verbose = false;
    bool quiet = false;
    app.add_flag("-v,--verbose", verbose, "Verbose (debug) logging");
    app.add_flag("-q,--quiet", quiet, "Warnings and errors only");

    app.add_subcommand("serve", "Run the daemon (default)");
    auto *ping = app.add_subcommand("ping", "Check that a running daemon is reachable");
    app.require_subcommand(0, 1);

    CLI11_PARSE(app, argc, argv);

    const auto level = verbose ? hestia::LogLevel::debug : quiet ? hestia::LogLevel::warn : hestia::LogLevel::info;

    // ping is a one-shot foreground tool — stderr only. The long-lived daemon
    // also logs to a rotated file, since the client detaches its stderr.
    if (ping->parsed()) {
        hestia::init_logging(level);
        return run_ping();
    }
    hestia::init_logging(level, hestia::ipc::runtime_dir() / "hestiad.log");
    return hestia::daemon::run_daemon();
}
