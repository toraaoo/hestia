include(GNUInstallDirs)

# hestia_engine — the launcher engine (config store, greeting, and the launcher
# logic to come). DAEMON-INTERNAL: only hestiad links this. Frontends reach this
# logic over the IPC bridge, never by linking it. (Until the CLI's config/greet
# commands move behind the daemon in a later phase, the CLI and desktop still
# link it directly.)
add_library(hestia_engine STATIC
        include/hestia/engine/engine.h
        include/hestia/engine/config_store.h
        include/hestia/engine/checksum.h
        include/hestia/engine/downloader.h
        include/hestia/greeting.h
        include/hestia/config.h
        src/engine.cc
        src/config_store.cc
        src/checksum.cc
        src/downloader.cc
        src/greeting.cc
        src/config.cc
)

# The engine resolves its data directory via hestia::paths (hestia/paths.h), the
# shared single source of truth for where Hestia's data lives.
target_link_libraries(hestia_engine PRIVATE hestia_shared)

target_include_directories(hestia_engine
        PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
        $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>
        PRIVATE
        src
)

# cpr stays PRIVATE: the downloader's public header exposes no cpr types.
target_link_libraries(hestia_engine PRIVATE
        fmt::fmt
        cpr::cpr
)

target_compile_options(hestia_engine PRIVATE
        $<$<CXX_COMPILER_ID:GNU,Clang>:-Wall;-Wextra;-Wpedantic>
        $<$<CXX_COMPILER_ID:MSVC>:/W4>
)

# "Development" component: kept out of end-user packages, which ship runtime only.
install(TARGETS hestia_engine
        ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR} COMPONENT Development
)
install(DIRECTORY include/ DESTINATION ${CMAKE_INSTALL_INCLUDEDIR} COMPONENT Development)
