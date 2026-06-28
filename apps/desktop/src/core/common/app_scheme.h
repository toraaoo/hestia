#pragma once
#include "include/cef_scheme.h"

namespace desktop::common {

void RegisterCustomSchemes(CefRawPtr<CefSchemeRegistrar> registrar);
void RegisterSchemeHandlerFactory();

// The base URL served by the scheme handler (e.g. "hestia://app/").
const char* GetSchemeOrigin();

}  // namespace desktop::common
