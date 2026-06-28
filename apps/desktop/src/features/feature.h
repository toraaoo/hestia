#pragma once
#include "core/ipc/ipc_router.h"

namespace desktop::features {

class Feature {
public:
    virtual ~Feature() = default;
    virtual const char* Name() const = 0;
    virtual void RegisterActions(ipc::Actions& on) = 0;
};

}  // namespace desktop::features
