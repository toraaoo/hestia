# CEF binary distribution version. Kept in its own file so CI can key the CEF
# download cache on it alone — unrelated desktop-CMake edits then don't trigger a
# ~1 GB re-download.
set(CEF_VERSION "149.0.6+g0d0eeb6+chromium-149.0.7827.201" CACHE STRING "CEF binary distribution version")
