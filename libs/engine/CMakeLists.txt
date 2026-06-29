include(GNUInstallDirs)

# hestia_engine — the launcher engine (config store, greeting, and the launcher
# logic to come). DAEMON-INTERNAL: only hestiad links this. Frontends reach this
# logic over the IPC bridge, never by linking it. (Until the CLI's config/greet
# commands move behind the daemon in a later phase, the CLI and desktop still
# link it directly.)
add_library(hestia_engine STATIC
        include/hestia/greeting.h
        include/hestia/config.h
        src/greeting.cc
        src/config.cc
)

target_include_directories(hestia_engine
        PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
        $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>
        PRIVATE
        src
)

target_link_libraries(hestia_engine PRIVATE
        fmt::fmt
)

target_compile_options(hestia_engine PRIVATE
        $<$<CXX_COMPILER_ID:GNU,Clang>:-Wall;-Wextra;-Wpedantic>
        $<$<CXX_COMPILER_ID:MSVC>:/W4>
)

install(TARGETS hestia_engine
        ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
)
install(DIRECTORY include/ DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})
