# Included once per generator with CPACK_GENERATOR set, so grouping differs
# between distro packages and installers.
if(CPACK_GENERATOR MATCHES "^(DEB|RPM)$")
    # One runtime-only package (daemon + cli + desktop), Development excluded.
    set(CPACK_COMPONENTS_GROUPING ALL_COMPONENTS_IN_ONE)
    set(CPACK_DEB_COMPONENT_INSTALL ON)
    set(CPACK_RPM_COMPONENT_INSTALL ON)
elseif(CPACK_GENERATOR STREQUAL "NSIS")
    set(CPACK_COMPONENTS_GROUPING IGNORE)
endif()
