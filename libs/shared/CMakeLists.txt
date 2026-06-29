include(GNUInstallDirs)

# Generate the shared identity header (name, id, vendor, version, channel).
configure_file(include/hestia/app_info.h.in
               "${CMAKE_CURRENT_BINARY_DIR}/include/hestia/app_info.h" @ONLY)

# hestia_shared — cross-cutting code linked by BOTH the daemon and every client
# (CLI/TUI/desktop): application identity and logging. It holds no
# launcher/engine logic — that lives in hestia_engine.
add_library(hestia_shared STATIC
        include/hestia/logging.h
        include/hestia/app_info.h.in
        src/logging.cc
)

target_include_directories(hestia_shared
        PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
        $<BUILD_INTERFACE:${CMAKE_CURRENT_BINARY_DIR}/include>
        $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>
)

# spdlog/fmt back logging only; they do not leak through the public headers.
target_link_libraries(hestia_shared PRIVATE
        spdlog::spdlog
        fmt::fmt
)

target_compile_options(hestia_shared PRIVATE
        $<$<CXX_COMPILER_ID:GNU,Clang>:-Wall;-Wextra;-Wpedantic>
        $<$<CXX_COMPILER_ID:MSVC>:/W4>
)

install(TARGETS hestia_shared
        ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
)
install(DIRECTORY include/ DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})
install(FILES "${CMAKE_CURRENT_BINARY_DIR}/include/hestia/app_info.h"
        DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}/hestia")
