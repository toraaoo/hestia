#if defined(_WIN32)
#include "include/cef_app.h"
#include "include/cef_sandbox_win.h"
#include "core/app/main_util.h"
#include <hestia/logging.h>
#include <windows.h>

int WINAPI wWinMain(HINSTANCE hInstance, HINSTANCE, PWSTR, int) {
    hestia::init_logging();

    void* sandbox_info = nullptr;
#if defined(CEF_USE_SANDBOX)
    CefScopedSandboxInfo scoped_sandbox;
    sandbox_info = scoped_sandbox.sandbox_info();
#endif

    CefMainArgs main_args(hInstance);
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
    CefString(&settings.locales_dir_path)   = exe + "\\locales";
    CefString(&settings.root_cache_path)    = exe + "\\cache";

    if (!CefInitialize(main_args, settings, app, sandbox_info))
        return CefGetExitCode();

    CefRunMessageLoop();
    CefShutdown();
    return 0;
}
#endif
