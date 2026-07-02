#include "core/app/main_util.h"
#include "core/app/browser_app.h"
#include "core/app/other_app.h"
#include "core/app/renderer_app.h"

#if defined(_WIN32)
#include <windows.h>
#elif !defined(__APPLE__)
#include <climits>
#include <unistd.h>
#endif

namespace desktop::app {

    CefRefPtr<CefCommandLine> CreateCommandLine(const CefMainArgs &main_args) {
        auto cmd = CefCommandLine::CreateCommandLine();
#if defined(_WIN32)
        cmd->InitFromString(::GetCommandLineW());
#else
        cmd->InitFromArgv(main_args.argc, main_args.argv);
#endif
        return cmd;
    }

    ProcessType GetProcessType(const CefRefPtr<CefCommandLine> &cmd) {
        if (!cmd->HasSwitch("type")) return ProcessType::browser;
        const auto type = cmd->GetSwitchValue("type");
        if (type == "renderer") return ProcessType::renderer;
#if !defined(_WIN32) && !defined(__APPLE__)
        // On Linux the zygote process forks into the other sub-process types and the
        // forked child inherits this app object. Its eventual type is unknown here,
        // so give the zygote the renderer app — otherwise forked renderers never get
        // a CefRenderProcessHandler and window.cefQuery is never injected (the
        // frontend then reports the bridge as "detached").
        if (type == "zygote") return ProcessType::renderer;
#endif
        return ProcessType::other;
    }

    CefRefPtr<CefApp> CreateApp(ProcessType type) {
        switch (type) {
        case ProcessType::browser: return new BrowserApp();
        case ProcessType::renderer: return new RendererApp();
        default: return new OtherApp();
        }
    }

    std::string GetExecutableDirectory() {
#if defined(_WIN32)
        char buf[MAX_PATH];
        GetModuleFileNameA(nullptr, buf, MAX_PATH);
        std::string path(buf);
        auto pos = path.rfind('\\');
        return pos == std::string::npos ? "." : path.substr(0, pos);
#elif defined(__APPLE__)
        return "."; // on macOS, paks live inside the framework; caller needn't set paths
#else
        char buf[PATH_MAX];
        ssize_t len = readlink("/proc/self/exe", buf, sizeof(buf) - 1);
        if (len == -1) return ".";
        buf[len] = '\0';
        std::string path(buf);
        auto pos = path.rfind('/');
        return pos == std::string::npos ? "." : path.substr(0, pos);
#endif
    }

} // namespace desktop::app
