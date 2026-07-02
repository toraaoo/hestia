#pragma once

// Each daemon feature area registers its channels onto the router in its own
// translation unit. Adding one is a new file plus one line in
// register_all_services below.
namespace hestia::daemon {
    class Router;

    void register_health_service(Router &router);
    void register_app_service(Router &router);
    void register_config_service(Router &router);
    void register_process_service(Router &router);
    void register_autostart_service(Router &router);
    void register_events_service(Router &router);
    void register_downloads_service(Router &router);

    // Wire every service onto the router. The serve loop calls this once; new
    // services are added here and nowhere else.
    inline void register_all_services(Router &router) {
        register_health_service(router);
        register_app_service(router);
        register_config_service(router);
        register_process_service(router);
        register_autostart_service(router);
        register_events_service(router);
        register_downloads_service(router);
    }
}
