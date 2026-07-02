#pragma once
#include "include/cef_app.h"
#include "include/cef_command_line.h"
#include <string>

namespace desktop::app {

    enum class ProcessType { browser, renderer, other };

    CefRefPtr<CefCommandLine> CreateCommandLine(const CefMainArgs &main_args);
    ProcessType GetProcessType(const CefRefPtr<CefCommandLine> &cmd);
    CefRefPtr<CefApp> CreateApp(ProcessType type);
    std::string GetExecutableDirectory();

} // namespace desktop::app
