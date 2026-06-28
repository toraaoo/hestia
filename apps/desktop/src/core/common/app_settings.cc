#include "core/common/app_settings.h"
#include "core/common/app_scheme.h"
#include "core/build_config.h"
#include <cstdlib>

namespace desktop::common {

namespace {
std::string g_dev_url;
}

void InitSettings(const CefRefPtr<CefCommandLine>& cmd) {
#if !defined(NDEBUG)
    if (cmd && cmd->HasSwitch("dev-url"))
        g_dev_url = cmd->GetSwitchValue("dev-url").ToString();
    else if (const char* e = std::getenv("HESTIA_DEV_URL"); e && *e)
        g_dev_url = e;
    else
        g_dev_url = APP_DEV_SERVER_URL;
#else
    (void)cmd;
    g_dev_url.clear();
#endif
}

std::string GetStartupURL() {
#if !defined(NDEBUG)
    if (!g_dev_url.empty()) return g_dev_url;
#endif
    return GetSchemeOrigin();
}

}  // namespace desktop::common
