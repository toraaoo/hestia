# Unit tests mirroring the source tree, linking only the shipped libraries.
add_executable(hestia_tests
        shared/protocol_test.cc
        shared/process_codec_test.cc
        shared/paths_test.cc
        engine/config_test.cc
        engine/config_store_test.cc
        engine/engine_test.cc
        engine/greeting_test.cc
)

target_link_libraries(hestia_tests PRIVATE
        hestia_shared
        hestia_engine
        GTest::gtest_main
)

target_compile_options(hestia_tests PRIVATE
        $<$<CXX_COMPILER_ID:GNU,Clang>:-Wall;-Wextra;-Wpedantic>
        $<$<CXX_COMPILER_ID:MSVC>:/W4>
)

include(GoogleTest)
gtest_discover_tests(hestia_tests)
