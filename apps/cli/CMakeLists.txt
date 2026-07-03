add_executable(hestia_cli)

target_sources(hestia_cli
        PRIVATE
        src/main.cc
        src/cli.cc
        src/command.cc
        src/output.cc
        src/registry.cc
        src/commands/java_command.cc
        src/commands/cache_command.cc
        src/commands/config_command.cc
        src/commands/daemon_command.cc
        src/commands/autostart_command.cc
)

target_include_directories(hestia_cli PRIVATE src)

target_link_libraries(hestia_cli PRIVATE
        hestia_shared
        CLI11::CLI11
        spdlog::spdlog
        ftxui::dom
)

set_target_properties(hestia_cli PROPERTIES OUTPUT_NAME "hestia")

install(TARGETS hestia_cli
        RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR} COMPONENT cli
)
