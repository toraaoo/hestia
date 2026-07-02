#include "core/app/main_util.h"
#include "include/cef_app.h"

#include <hestia/logging.h>
#include <hestia/paths.h>

int main(int argc, char *argv[]) {
    hestia::init_logging();

    CefMainArgs main_args(argc, argv);
    auto cmd = desktop::app::CreateCommandLine(main_args);
    auto app = desktop::app::CreateApp(desktop::app::GetProcessType(cmd));

    // Sub-processes return here; the browser process continues.
    int code = CefExecuteProcess(main_args, app, nullptr);
    if (code >= 0) return code;

    CefSettings settings;
    // On Linux the sandbox lives inside libcef.so; no separate lib to link.
    // Set no_sandbox=true in Debug where USE_SANDBOX is off.
#if !defined(CEF_USE_SANDBOX)
    settings.no_sandbox = true;
#endif

    const std::string exe = desktop::app::GetExecutableDirectory();
    CefString(&settings.resources_dir_path) = exe;
    CefString(&settings.locales_dir_path) = exe + "/locales";
    // Writable per-user cache; the exe dir is read-only (AppImage / system install).
    CefString(&settings.root_cache_path) = (hestia::paths::data_home() / "cache").string();

    if (!CefInitialize(main_args, settings, app, nullptr)) return CefGetExitCode();

    CefRunMessageLoop();
    CefShutdown();
    return 0;
}
