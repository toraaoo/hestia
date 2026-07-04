include(GNUInstallDirs)

# Generate the shared identity header (name, id, vendor, version, channel).
configure_file(include/hestia/app_info.h.in
               "${CMAKE_CURRENT_BINARY_DIR}/include/hestia/app_info.h" @ONLY)

# hestia_shared — cross-cutting code linked by BOTH the daemon and every client
# (CLI/TUI/desktop): the IPC transport + protocol machinery, the typed wire
# contracts (hestia::proto), the client SDK, application identity, and logging.
# It holds no launcher/engine logic — that lives in hestia_engine (daemon-only).
add_library(hestia_shared STATIC
        include/hestia/logging.h
        include/hestia/paths.h
        include/hestia/app_info.h.in
        include/hestia/ipc/transport.h
        include/hestia/ipc/endpoint.h
        include/hestia/ipc/protocol.h
        include/hestia/ipc/errors.h
        include/hestia/proto/contract.h
        include/hestia/proto/health.h
        include/hestia/proto/app.h
        include/hestia/proto/daemon.h
        include/hestia/proto/config.h
        include/hestia/proto/events.h
        include/hestia/proto/process.h
        include/hestia/proto/download.h
        include/hestia/proto/java.h
        include/hestia/proto/cache.h
        include/hestia/client.h
        include/hestia/client/facade.h
        include/hestia/client/app.h
        include/hestia/client/daemon.h
        include/hestia/client/config.h
        include/hestia/client/process.h
        include/hestia/client/download.h
        include/hestia/client/java.h
        include/hestia/client/cache.h
        include/hestia/client/bridge.h
        src/logging.cc
        src/paths.cc
        src/endpoint.cc
        src/transport_posix.cc
        src/transport_windows.cc
        src/protocol.cc
        src/proto/process.cc
        src/proto/download.cc
        src/proto/java.cc
        src/client/session.h
        src/client/session.cc
        src/client/spawn.h
        src/client/spawn.cc
        src/client/client.cc
        src/client/app.cc
        src/client/daemon.cc
        src/client/config.cc
        src/client/process.cc
        src/client/download.cc
        src/client/java.cc
        src/client/cache.cc
        src/client/bridge.cc
)

target_include_directories(hestia_shared
        PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
        $<BUILD_INTERFACE:${CMAKE_CURRENT_BINARY_DIR}/include>
        $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>
)

# nlohmann/json is PUBLIC: it appears in the protocol envelope and the proto
# contract headers, so consumers need its headers. spdlog/fmt back logging only
# and do not leak through the public headers, so they stay PRIVATE.
target_link_libraries(hestia_shared
        PUBLIC
        nlohmann_json::nlohmann_json
        PRIVATE
        spdlog::spdlog
        fmt::fmt
        Threads::Threads
        $<$<PLATFORM_ID:Windows>:advapi32> # token/SID APIs for the pipe DACL
)

target_compile_options(hestia_shared PRIVATE
        $<$<CXX_COMPILER_ID:GNU,Clang>:-Wall;-Wextra;-Wpedantic>
        $<$<CXX_COMPILER_ID:MSVC>:/W4>
)

# Debug builds keep all data inside the checkout (see paths.cc).
target_compile_definitions(hestia_shared PRIVATE
        $<$<CONFIG:Debug>:HESTIA_DEV_HOME="${CMAKE_SOURCE_DIR}/.hestia">
)

# "Development" component: kept out of end-user packages, which ship runtime only.
install(TARGETS hestia_shared
        ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR} COMPONENT Development
)
install(DIRECTORY include/ DESTINATION ${CMAKE_INSTALL_INCLUDEDIR} COMPONENT Development)
install(FILES "${CMAKE_CURRENT_BINARY_DIR}/include/hestia/app_info.h"
        DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}/hestia" COMPONENT Development)
