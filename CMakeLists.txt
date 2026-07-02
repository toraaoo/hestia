cmake_minimum_required(VERSION 3.21)

project(hestia
        VERSION 0.0.1
        LANGUAGES C CXX
)

if(CMAKE_CONFIGURATION_TYPES)
    set(CMAKE_RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/$<CONFIG>")
    set(CMAKE_LIBRARY_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/$<CONFIG>")
    set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/$<CONFIG>")
else()
    set(CMAKE_RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/${CMAKE_BUILD_TYPE}")
    set(CMAKE_LIBRARY_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/${CMAKE_BUILD_TYPE}")
    set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/${CMAKE_BUILD_TYPE}")
endif()

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Windows: CEF links the static MSVC runtime (/MT), so build the whole project
# against it. Mixing /MT (CEF, desktop) with the default /MD (spdlog, fmt, …) in
# one link fails with unresolved __imp__ CRT symbols.
if(MSVC)
    set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>")
endif()

list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_SOURCE_DIR}/cmake")

# The daemon supervises on background threads and the client SDK runs a reader
# thread, so both link the platform threads library.
set(THREADS_PREFER_PTHREAD_FLAG ON)
find_package(Threads REQUIRED)

# ---- Compiler cache ----------------------------------------------------------
# Route compiles through ccache when it is installed, so rebuilds reuse cached
# object files. On by default and a no-op when ccache is absent.
option(USE_CCACHE "Route compiles through ccache when available" ON)
if(USE_CCACHE)
    find_program(CCACHE_PROGRAM ccache)
    if(CCACHE_PROGRAM)
        message(STATUS "Using ccache: ${CCACHE_PROGRAM}")
        set(CMAKE_C_COMPILER_LAUNCHER   "${CCACHE_PROGRAM}")
        set(CMAKE_CXX_COMPILER_LAUNCHER "${CCACHE_PROGRAM}")
    endif()
endif()

# ---- Shared application identity (consumed by hestia_shared + desktop) ------
set(APP_NAME    "Hestia"                    CACHE STRING "Human-facing product name")
set(APP_ID      "tech.lawrenceallen.hestia" CACHE STRING "Reverse-DNS application id")
set(APP_VENDOR  "toraaoo"                   CACHE STRING "Vendor / maintainer")
set(APP_CHANNEL "dev"                       CACHE STRING "Release channel")
# APP_VERSION flows from project(hestia VERSION ...) above.

# ---- Front-end selection -----------------------------------------------------
# The daemon and tray are the resident core and are always built. The front-ends
# are opt-out, so a focused dev build can skip the expensive ones — chiefly the
# desktop launcher, which pulls in CEF (a ~1 GB configure-time download and a
# long link). Vendored deps are EXCLUDE_FROM_ALL (see third_party/), so skipping
# a front-end transitively skips its dependencies too.
option(BUILD_DESKTOP "Build the desktop launcher (CEF)" ON)
option(BUILD_CLI     "Build the CLI front-end"          ON)

# ---- Tests -------------------------------------------------------------------
# Defined before third_party so the vendored GoogleTest is gated on it.
option(BUILD_TESTING "Build the unit tests" ON)
if(BUILD_TESTING)
    enable_testing()
endif()

add_subdirectory(third_party)
add_subdirectory(libs/shared)
add_subdirectory(libs/engine)

if(BUILD_CLI)
    add_subdirectory(apps/cli)
endif()
if(BUILD_DESKTOP)
    add_subdirectory(apps/desktop)
endif()

add_subdirectory(apps/daemon)
add_subdirectory(apps/tray)

if(BUILD_TESTING)
    add_subdirectory(tests)
endif()

# Release artifacts (CPack): portable archives, DEB/RPM, NSIS/WiX installers.
include(Packaging)
