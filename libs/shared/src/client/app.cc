#include "hestia/client/app.h"

#include "session.h"

namespace hestia::client {
    proto::AppInfo::Result App::info() {
        return session_->call<proto::AppInfo>();
    }
} // namespace hestia::client
