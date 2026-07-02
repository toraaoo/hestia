#if defined(_WIN32)
#include "core/app/main_util.h"
#include "include/cef_app.h"
#include <hestia/logging.h>
#include <hestia/paths.h>
#include <windows.h>

#if defined(CEF_USE_BOOTSTRAP)
#include "include/cef_sandbox_win.h"
#endif

namespace {
    int RunMain(CefMainArgs main_args, void *sandbox_info) {
        hestia::init_logging();

        auto cmd = desktop::app::CreateCommandLine(main_args);
        auto app = desktop::app::CreateApp(desktop::app::GetProcessType(cmd));

        int code = CefExecuteProcess(main_args, app, sandbox_info);
        if (code >= 0) return code;

        CefSettings settings;
#if !defined(CEF_USE_SANDBOX)
        settings.no_sandbox = true;
#endif

        const std::string exe = desktop::app::GetExecutableDirectory();
        CefString(&settings.resources_dir_path) = exe;
        CefString(&settings.locales_dir_path) = exe + "\\locales";
        // Writable per-user cache; Program Files is read-only.
        CefString(&settings.root_cache_path) = (hestia::paths::data_home() / "cache").string();

        if (!CefInitialize(main_args, settings, app, sandbox_info)) return CefGetExitCode();

        CefRunMessageLoop();
        CefShutdown();
        return 0;
    }
} // namespace

#if defined(CEF_USE_BOOTSTRAP)
// Sandbox builds run as a DLL; bootstrap.exe (renamed to the app binary) calls
// this exported entry point and supplies the sandbox info it owns.
CEF_BOOTSTRAP_EXPORT int RunWinMain(HINSTANCE hInstance, LPTSTR, int, void *sandbox_info, cef_version_info_t *) {
    return RunMain(CefMainArgs(hInstance), sandbox_info);
}
#else
int WINAPI wWinMain(HINSTANCE hInstance, HINSTANCE, PWSTR, int) {
    return RunMain(CefMainArgs(hInstance), nullptr);
}
#endif
#endif
