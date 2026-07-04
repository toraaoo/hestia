# EXCLUDE_FROM_ALL: vendored deps build only when a first-party target links
# them, not as part of the default `all` target.
add_subdirectory(fmt EXCLUDE_FROM_ALL)

# Point spdlog at the vendored fmt above instead of its bundled copy, so a target
# linking both spdlog and fmt gets one fmt (no duplicate-symbol link errors).
set(SPDLOG_FMT_EXTERNAL ON CACHE BOOL "" FORCE)
add_subdirectory(spdlog EXCLUDE_FROM_ALL)

# CLI11: header-only command-line parser. Keep its tests/examples/docs and
# install rules out of our build.
set(CLI11_BUILD_TESTS OFF CACHE BOOL "" FORCE)
set(CLI11_BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
set(CLI11_BUILD_DOCS OFF CACHE BOOL "" FORCE)
set(CLI11_INSTALL OFF CACHE BOOL "" FORCE)
add_subdirectory(CLI11 EXCLUDE_FROM_ALL)

# nlohmann/json: header-only JSON for the daemon protocol envelope. Keep its
# tests and install rules out of our build.
set(JSON_BuildTests OFF CACHE BOOL "" FORCE)
set(JSON_Install OFF CACHE BOOL "" FORCE)
add_subdirectory(json EXCLUDE_FROM_ALL)

# cpr: HTTP client for the engine's downloader. Builds its bundled curl
# (fetched at configure time — the CEF-download precedent); tests stay out of
# our build. BUILD_EXAMPLES is curl's own option (defaults ON) — without this
# curl configures ~150 example executables that each emit a runtime-search-path
# warning against our build dir. The Downloader only does plain HTTP/HTTPS
# file downloads (no cookies), so curl's opportunistic system-lib features are
# pinned off: left auto-detected, they vary per machine, and a static build
# under-links libidn2 (missing its own libunistring dependency), breaking the
# link on systems that have libidn2 installed. PSL (cookie-domain security) is
# pinned off too — it drags in a libpsl meson fetch with the same libidn2
# auto-detection. cpr defaults BUILD_SHARED_LIBS on; pinned OFF so cpr and its
# curl link statically — otherwise every binary needs the curl DLL/so on its
# search path (0xc0000135 at gtest discovery on Windows).
set(BUILD_SHARED_LIBS OFF CACHE BOOL "" FORCE)
set(CPR_BUILD_TESTS OFF CACHE BOOL "" FORCE)
set(CPR_BUILD_TESTS_SSL OFF CACHE BOOL "" FORCE)
set(CPR_CURL_USE_LIBPSL OFF CACHE BOOL "" FORCE)
set(BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
set(USE_LIBIDN2 OFF CACHE BOOL "" FORCE)
set(CURL_USE_LIBSSH2 OFF CACHE BOOL "" FORCE)
set(CURL_BROTLI OFF CACHE STRING "" FORCE)
set(CURL_ZSTD OFF CACHE STRING "" FORCE)
add_subdirectory(cpr EXCLUDE_FROM_ALL)

# FTXUI: terminal UI elements (progress gauges for the CLI, the future TUI).
set(FTXUI_BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
set(FTXUI_BUILD_DOCS OFF CACHE BOOL "" FORCE)
set(FTXUI_BUILD_TESTS OFF CACHE BOOL "" FORCE)
set(FTXUI_ENABLE_INSTALL OFF CACHE BOOL "" FORCE)
add_subdirectory(ftxui EXCLUDE_FROM_ALL)

# GoogleTest, built only when BUILD_TESTING is on. gtest_force_shared_crt stays
# OFF so gtest matches the project's static MSVC runtime (/MT, set for CEF).
if(BUILD_TESTING)
    set(BUILD_GMOCK OFF CACHE BOOL "" FORCE)
    set(INSTALL_GTEST OFF CACHE BOOL "" FORCE)
    set(gtest_force_shared_crt OFF CACHE BOOL "" FORCE)
    add_subdirectory(googletest EXCLUDE_FROM_ALL)
endif()
