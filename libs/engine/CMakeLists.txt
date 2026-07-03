include(GNUInstallDirs)

# hestia_engine — the launcher engine. Daemon-internal: frontends reach it over
# IPC, never by linking it (the desktop still links it, a transitional exception).
add_library(hestia_engine STATIC
        include/hestia/engine/engine.h
        include/hestia/engine/config.h
        include/hestia/engine/downloader.h
        include/hestia/engine/greeting.h
        src/engine.cc
        src/config.cc
        src/download/checksum.h
        src/download/checksum.cc
        src/download/downloader.cc
        src/greeting/greeting.cc
)

# PUBLIC: shared types (ipc::Checksum) appear in the engine's public headers.
target_link_libraries(hestia_engine PUBLIC hestia_shared)

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
