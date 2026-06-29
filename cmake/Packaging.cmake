# Packaging.cmake — CPack configuration for Hestia's release artifacts.
#
# Generators (selected per platform, or via `cpack -G`):
#   Linux   : TGZ (portable), DEB, RPM   + AppImage (packaging/appimage/build-appimage.sh)
#   Windows : ZIP (portable), NSIS (.exe), WIX (.msi)
#
# Component model:
#   cli         default, required  — hestia (CLI/TUI) + hestiad (daemon)
#   desktop     optional           — desktop launcher + tray + CEF runtime
#   Development  (libs/headers)     — never packaged
#
# Archives, DEB and RPM are monolithic (everything in one); NSIS and WiX present
# the component picker so "CLI only" is the default and desktop is opt-in.

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

# ---- Windows: NSIS / WiX ----------------------------------------------------
if(WIN32)
    set(CPACK_NSIS_PACKAGE_NAME "Hestia")
    set(CPACK_NSIS_DISPLAY_NAME "Hestia")
    set(CPACK_NSIS_ENABLE_UNINSTALL_BEFORE_INSTALL ON)
    set(CPACK_NSIS_MODIFY_PATH ON)
    # DPI-aware installer + uninstaller (otherwise blurry on HiDPI displays).
    set(CPACK_NSIS_MANIFEST_DPI_AWARE TRUE)
    set(CPACK_NSIS_MUI_ICON     "${CMAKE_SOURCE_DIR}/packaging/icons/hestia.ico")
    set(CPACK_NSIS_MUI_UNIICON  "${CMAKE_SOURCE_DIR}/packaging/icons/hestia.ico")
    set(CPACK_NSIS_INSTALLED_ICON_NAME "bin\\\\hestia.exe")
    set(CPACK_NSIS_MENU_LINKS "desktop\\\\HestiaLauncher.exe" "Hestia")

    set(CPACK_WIX_VERSION 3)
    set(CPACK_WIX_UPGRADE_GUID "A02B98DA-53D3-4413-92B1-47C984829628")
    set(CPACK_WIX_PRODUCT_ICON "${CMAKE_SOURCE_DIR}/packaging/icons/hestia.ico")
    set(CPACK_WIX_LICENSE_RTF  "${CMAKE_SOURCE_DIR}/packaging/windows/license.rtf")
    set(CPACK_WIX_PATCH_FILE   "${CMAKE_SOURCE_DIR}/packaging/windows/wix-path-patch.xml")

    # MSI Start-menu shortcut for the desktop launcher. NSIS gets its shortcut via
    # CPACK_NSIS_MENU_LINKS above; the WiX generator needs this INSTALL-file
    # property instead. Tied to the desktop component, so it only appears when the
    # desktop feature is selected.
    set_property(INSTALL "desktop/${APP_BINARY_NAME}.exe"
                 PROPERTY CPACK_START_MENU_SHORTCUTS "Hestia")
endif()

if(WIN32)
    set(CPACK_GENERATOR "ZIP;NSIS;WIX")
elseif(UNIX AND NOT APPLE)
    set(CPACK_GENERATOR "TGZ;DEB;RPM")
endif()

include(CPack)
