#pragma once
#include <string>
#include "include/cef_command_line.h"

namespace desktop::common {

// Read the dev-server URL (Debug-only) from the command line or environment.
// In Release builds this is a no-op; GetStartupURL() always returns the scheme.
void InitSettings(const CefRefPtr<CefCommandLine>& cmd);

// URL the browser opens on startup:
//   Debug  — dev-server if set, otherwise the embedded scheme.
//   Release — always the embedded scheme.
std::string GetStartupURL();

}  // namespace desktop::common
