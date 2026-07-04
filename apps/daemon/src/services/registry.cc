#include "services/registry.h"

#include "services/app_service.h"
#include "services/cache_service.h"
#include "services/config_service.h"
#include "services/daemon_service.h"
#include "services/downloads_service.h"
#include "services/events_service.h"
#include "services/health_service.h"
#include "services/java_service.h"
#include "services/process_service.h"

namespace hestia::daemon {
    std::vector<std::unique_ptr<Service>> make_services() {
        std::vector<std::unique_ptr<Service>> services;
        services.push_back(std::make_unique<HealthService>());
        services.push_back(std::make_unique<AppService>());
        services.push_back(std::make_unique<DaemonService>());
        services.push_back(std::make_unique<ConfigService>());
        services.push_back(std::make_unique<ProcessService>());
        services.push_back(std::make_unique<EventsService>());
        services.push_back(std::make_unique<DownloadsService>());
        services.push_back(std::make_unique<JavaService>());
        services.push_back(std::make_unique<CacheService>());
        return services;
    }
} // namespace hestia::daemon
