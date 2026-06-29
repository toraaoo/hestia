# Per-generator packaging tweaks. CPack includes this once per generator with
# CPACK_GENERATOR set, so grouping/prefix can differ between archive and
# installer generators. Without component install the archive/deb/rpm generators
# fall back to monolithic mode, which installs *every* component — including the
# Development files we want to keep out of end-user packages.
if(CPACK_GENERATOR MATCHES "^(TGZ|TBZ2|TXZ|ZIP|DEB|RPM)$")
    # One runtime-only package: cli + desktop merged, Development excluded.
    set(CPACK_COMPONENTS_GROUPING ALL_COMPONENTS_IN_ONE)
    set(CPACK_ARCHIVE_COMPONENT_INSTALL ON)
    set(CPACK_DEB_COMPONENT_INSTALL ON)
    set(CPACK_RPM_COMPONENT_INSTALL ON)
endif()

# Portable archives live at their own root (bin/, lib/, share/); distro packages
# install under /usr.
if(CPACK_GENERATOR MATCHES "^(TGZ|TBZ2|TXZ|ZIP)$")
    set(CPACK_PACKAGING_INSTALL_PREFIX "/")
elseif(CPACK_GENERATOR MATCHES "^(NSIS|WIX)$")
    # Component picker: CLI default/required, desktop opt-in.
    set(CPACK_COMPONENTS_GROUPING IGNORE)
endif()
