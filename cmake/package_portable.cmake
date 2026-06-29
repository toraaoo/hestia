# Flat portable archive: executables + CEF runtime at the archive root, since
# CPack's archive layout is the FHS tree it shares with DEB/RPM. tar.gz on Unix,
# zip on Windows; version read from project().
#
#   cmake -DBUILD_DIR=build -DOUT_DIR=<abs> -DSOURCE_DIR=<abs> -P package_portable.cmake
cmake_minimum_required(VERSION 3.21)

foreach(_req BUILD_DIR OUT_DIR SOURCE_DIR)
    if(NOT DEFINED ${_req})
        message(FATAL_ERROR "package_portable: ${_req} is required")
    endif()
endforeach()

file(READ "${SOURCE_DIR}/CMakeLists.txt" _top)
string(REGEX MATCH "VERSION[ \t\r\n]+([0-9]+\\.[0-9]+\\.[0-9]+)" _ "${_top}")
set(_version "${CMAKE_MATCH_1}")
if(CMAKE_HOST_WIN32)
    set(_os windows)
else()
    set(_os linux)
endif()

set(_name "hestia-${_version}-${_os}-x86_64")
set(_stage "${BUILD_DIR}/_portable")
set(_inst "${_stage}/inst")
set(_root "${_stage}/${_name}")
file(REMOVE_RECURSE "${_stage}")
file(MAKE_DIRECTORY "${_root}")

foreach(_comp daemon cli desktop)
    execute_process(
        COMMAND ${CMAKE_COMMAND} --install "${BUILD_DIR}" --prefix "${_inst}" --component ${_comp}
        RESULT_VARIABLE _rc)
    if(NOT _rc EQUAL 0)
        message(FATAL_ERROR "package_portable: installing component '${_comp}' failed")
    endif()
endforeach()

# CLI/daemon/tray stay in bin/; the desktop launcher + CEF runtime go to the
# archive root so the app is the obvious thing to double-click.
if(EXISTS "${_inst}/bin")
    file(COPY "${_inst}/bin" DESTINATION "${_root}")
endif()
foreach(_dir "${_inst}/lib/hestia" "${_inst}/desktop")
    if(EXISTS "${_dir}")
        file(GLOB _entries "${_dir}/*")
        file(COPY ${_entries} DESTINATION "${_root}")
    endif()
endforeach()

file(MAKE_DIRECTORY "${OUT_DIR}")
if(CMAKE_HOST_WIN32)
    set(_archive "${OUT_DIR}/${_name}.zip")
    execute_process(COMMAND ${CMAKE_COMMAND} -E tar cf "${_archive}" --format=zip "${_name}"
                    WORKING_DIRECTORY "${_stage}" RESULT_VARIABLE _rc)
else()
    set(_archive "${OUT_DIR}/${_name}.tar.gz")
    execute_process(COMMAND ${CMAKE_COMMAND} -E tar czf "${_archive}" "${_name}"
                    WORKING_DIRECTORY "${_stage}" RESULT_VARIABLE _rc)
endif()
if(NOT _rc EQUAL 0)
    message(FATAL_ERROR "package_portable: archive creation failed")
endif()
message(STATUS "portable archive: ${_archive}")
