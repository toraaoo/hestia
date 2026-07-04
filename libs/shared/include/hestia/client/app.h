#pragma once

#include <hestia/client/facade.h>
#include <hestia/proto/app.h>

namespace hestia::client {
    class App : public Facade {
    public:
        using Facade::Facade;

        proto::AppInfo::Result info();
    };
} // namespace hestia::client
