# Release post-build step: delete all locale .pak files except those listed in
# KEEP. Called via cmake -P from apps/desktop/CMakeLists.txt.
#
# Variables (passed via -D):
#   LOCALES_DIR  path to the locales/ directory
#   KEEP         semicolon-separated list of files to keep, e.g. "en-US.pak"

file(GLOB all_paks "${LOCALES_DIR}/*.pak")
foreach(pak ${all_paks})
    get_filename_component(name "${pak}" NAME)
    # list(FIND) instead of the IN_LIST operator keeps this policy-independent
    # (no CMP0057 needed) when run standalone via `cmake -P`.
    list(FIND KEEP "${name}" keep_index)
    if(keep_index EQUAL -1)
        file(REMOVE "${pak}")
    endif()
endforeach()
