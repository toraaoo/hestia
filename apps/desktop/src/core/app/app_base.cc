#include "core/app/app_base.h"
#include "core/common/app_scheme.h"

namespace desktop::app {

void AppBase::OnRegisterCustomSchemes(CefRawPtr<CefSchemeRegistrar> registrar) {
    common::RegisterCustomSchemes(registrar);
}

}  // namespace desktop::app
