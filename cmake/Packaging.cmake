# Packaging.cmake — CPack config for the distro packages and Windows installer.
# Portable archives are built separately (cmake/package_portable.cmake); AppImage
# via packaging/appimage/build-appimage.sh.
#
#   Linux   : DEB, RPM
#   Windows : NSIS (.exe)
#
# Component model:
#   daemon      required           — hestiad
#   cli                            — hestia (CLI/TUI)
#   desktop     optional           — desktop launcher + tray + CEF runtime
#   Development  (libs/headers)     — never packaged
#
# DEB/RPM are monolithic (all runtime components in one); NSIS presents the
# component picker so "CLI only" is the default and desktop is opt-in.

include(GNUInstallDirs)

set(CPACK_PACKAGE_NAME    "hestia")
set(CPACK_PACKAGE_VENDOR  "${APP_VENDOR}")
set(CPACK_PACKAGE_VERSION "${PROJECT_VERSION}")
set(CPACK_PACKAGE_CONTACT "${APP_VENDOR}")
set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "Fast, lightweight Minecraft launcher (CLI/TUI + desktop)")
set(CPACK_RESOURCE_FILE_LICENSE "${CMAKE_SOURCE_DIR}/LICENSE")
set(CPACK_PACKAGE_INSTALL_DIRECTORY "Hestia")
set(CPACK_VERBATIM_VARIABLES TRUE)
set(CPACK_STRIP_FILES TRUE)

# Grouping/component-install differs per generator (see the file).
set(CPACK_PROJECT_CONFIG_FILE "${CMAKE_SOURCE_DIR}/cmake/CPackOptions.cmake")

# Runtime components. The daemon is the resident core every front-end needs; the
# CLI and desktop are separately selectable on top of it.
set(_hestia_components daemon cli)
if(BUILD_DESKTOP)
    list(APPEND _hestia_components desktop)
endif()
set(CPACK_COMPONENTS_ALL ${_hestia_components})

set(CPACK_COMPONENT_DAEMON_DISPLAY_NAME "Daemon")
set(CPACK_COMPONENT_DAEMON_DESCRIPTION  "The hestiad background daemon.")
set(CPACK_COMPONENT_DAEMON_REQUIRED TRUE)
set(CPACK_COMPONENT_DAEMON_HIDDEN TRUE)

set(CPACK_COMPONENT_CLI_DISPLAY_NAME "Command-line tools")
set(CPACK_COMPONENT_CLI_DESCRIPTION  "The hestia CLI and TUI.")
set(CPACK_COMPONENT_CLI_DEPENDS daemon)

set(CPACK_COMPONENT_DESKTOP_DISPLAY_NAME "Desktop launcher")
set(CPACK_COMPONENT_DESKTOP_DESCRIPTION  "Graphical desktop launcher and system-tray helper.")
set(CPACK_COMPONENT_DESKTOP_DEPENDS daemon)
set(CPACK_COMPONENT_DESKTOP_DISABLED TRUE)

# Portable archive naming: hestia-<version>-<os>-x86_64.{tar.gz,zip}
string(TOLOWER "${CMAKE_SYSTEM_NAME}" _os)
set(CPACK_ARCHIVE_FILE_NAME "hestia-${PROJECT_VERSION}-${_os}-x86_64")

# ---- Linux: DEB / RPM -------------------------------------------------------
if(UNIX AND NOT APPLE)
    set(CPACK_PACKAGING_INSTALL_PREFIX "/usr")

    set(CPACK_DEBIAN_PACKAGE_MAINTAINER "${APP_VENDOR}")
    set(CPACK_DEBIAN_PACKAGE_SECTION "games")
    set(CPACK_DEBIAN_PACKAGE_SHLIBDEPS ON)
    set(CPACK_DEBIAN_FILE_NAME DEB-DEFAULT)
    set(CPACK_DEBIAN_PACKAGE_CONTROL_EXTRA "${CMAKE_SOURCE_DIR}/packaging/linux/postinst")

    set(CPACK_RPM_PACKAGE_LICENSE "MIT")
    set(CPACK_RPM_PACKAGE_GROUP "Amusements/Games")
    set(CPACK_RPM_PACKAGE_AUTOREQ ON)
    set(CPACK_RPM_FILE_NAME RPM-DEFAULT)
    set(CPACK_RPM_POST_INSTALL_SCRIPT_FILE "${CMAKE_SOURCE_DIR}/packaging/linux/postinst")
    # Don't claim ownership of system dirs the distro already provides.
    set(CPACK_RPM_EXCLUDE_FROM_AUTO_FILELIST_ADDITION
        /usr/lib /usr/bin /usr/share /usr/share/applications /usr/share/icons
        /usr/share/icons/hicolor /usr/share/icons/hicolor/scalable
        /usr/share/icons/hicolor/scalable/apps)
endif()

# ---- Windows: NSIS ----------------------------------------------------------
if(WIN32)
    set(CPACK_NSIS_PACKAGE_NAME "Hestia")
    set(CPACK_NSIS_DISPLAY_NAME "Hestia")
    set(CPACK_NSIS_ENABLE_UNINSTALL_BEFORE_INSTALL ON)
    set(CPACK_NSIS_MANIFEST_DPI_AWARE TRUE)
    set(CPACK_NSIS_MUI_ICON     "${CMAKE_SOURCE_DIR}/packaging/icons/hestia.ico")
    set(CPACK_NSIS_MUI_UNIICON  "${CMAKE_SOURCE_DIR}/packaging/icons/hestia.ico")
    set(CPACK_NSIS_INSTALLED_ICON_NAME "bin\\\\hestia.exe")
    set(CPACK_NSIS_MENU_LINKS "desktop\\\\${APP_BINARY_NAME}.exe" "Hestia")

    # Put the CLI on PATH via the EnVar plugin. CPACK_NSIS_MODIFY_PATH's built-in
    # macro overflows NSIS's 1024-char string limit when the system PATH is long;
    # EnVar reads/writes the registry directly and is duplicate-safe. The plugin
    # DLL must be in the NSIS plugins dir (the Windows CI job installs it).
    set(CPACK_NSIS_EXTRA_INSTALL_COMMANDS
        "EnVar::SetHKLM\n  EnVar::AddValueEx 'Path' '$INSTDIR\\\\bin'\n  Pop $0")
    set(CPACK_NSIS_EXTRA_UNINSTALL_COMMANDS
        "EnVar::SetHKLM\n  EnVar::DeleteValue 'Path' '$INSTDIR\\\\bin'\n  Pop $0")
endif()

# Portable archives are built by cmake/package_portable.cmake; CPack drives the
# installers and distro packages.
if(WIN32)
    set(CPACK_GENERATOR "NSIS")
elseif(UNIX AND NOT APPLE)
    set(CPACK_GENERATOR "DEB;RPM")
endif()

include(CPack)
