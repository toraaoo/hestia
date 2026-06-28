#pragma once
#include "core/app/app_base.h"

namespace desktop::app {

// GPU and utility sub-processes only need the custom scheme registered;
// AppBase already does that, so OtherApp has no additional logic.
class OtherApp : public AppBase {
private:
    IMPLEMENT_REFCOUNTING(OtherApp);
};

}  // namespace desktop::app
