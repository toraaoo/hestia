include(GNUInstallDirs)

# hestiad — the Hestia daemon. Owns the IPC endpoint, the request router, and
# (in later phases) the process supervisor and persisted process table. Links
# hestia_engine: the daemon is the only thing that links the engine directly.
add_executable(hestia_daemon
        src/main.cc
        src/runtime/server.cc
        src/runtime/server.h
        src/runtime/runtime.h
        src/runtime/router.cc
        src/runtime/router.h
        src/runtime/channels.h
        src/runtime/handler_context.h
        src/runtime/event_hub.h
        src/process/process_supervisor.cc
        src/process/process_supervisor.h
        src/process/process_table.cc
        src/process/process_table.h
        src/process/process_spawner.cc
        src/process/process_spawner.h
        src/process/liveness_probe.cc
        src/process/liveness_probe.h
        src/process/log_streamer.cc
        src/process/log_streamer.h
        src/process/restart_policy.h
        src/process/process_types.h
        src/downloads/download_manager.cc
        src/downloads/download_manager.h
        src/java/install_manager.cc
        src/java/install_manager.h
        src/platform/autostart.cc
        src/platform/autostart.h
        src/platform/win_util.h
        src/services/service.h
        src/services/registry.cc
        src/services/registry.h
        src/services/health_service.cc
        src/services/health_service.h
        src/services/app_service.cc
        src/services/app_service.h
        src/services/daemon_service.cc
        src/services/daemon_service.h
        src/services/config_service.cc
        src/services/config_service.h
        src/services/process_service.cc
        src/services/process_service.h
        src/services/events_service.cc
        src/services/events_service.h
        src/services/downloads_service.cc
        src/services/downloads_service.h
        src/services/java_service.cc
        src/services/java_service.h
        src/services/cache_service.cc
        src/services/cache_service.h
)

target_include_directories(hestia_daemon PRIVATE src)

target_link_libraries(hestia_daemon PRIVATE
        hestia_shared
        hestia_engine
        nlohmann_json::nlohmann_json
        CLI11::CLI11
        spdlog::spdlog
        Threads::Threads
)

target_compile_options(hestia_daemon PRIVATE
        $<$<CXX_COMPILER_ID:GNU,Clang>:-Wall;-Wextra;-Wpedantic>
        $<$<CXX_COMPILER_ID:MSVC>:/W4>
)

set_target_properties(hestia_daemon PROPERTIES OUTPUT_NAME "hestiad")

# The daemon is the resident core every front-end needs, so it's its own
# required component (CLI and desktop both depend on it).
install(TARGETS hestia_daemon
        RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR} COMPONENT daemon
)
