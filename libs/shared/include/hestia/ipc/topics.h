#pragma once

// The protocol's event-topic vocabulary in one place, shared by the daemon
// (which publishes) and the client (which matches).
namespace hestia::ipc::topics {
    inline constexpr const char *kProcessState = "process.state";
    inline constexpr const char *kProcessLog = "process.log";
    inline constexpr const char *kDownloadProgress = "download.progress";
    inline constexpr const char *kDownloadDone = "download.done";
    inline constexpr const char *kDownloadError = "download.error";
    inline constexpr const char *kJavaInstallProgress = "java.install.progress";
    inline constexpr const char *kJavaInstallDone = "java.install.done";
    inline constexpr const char *kJavaInstallError = "java.install.error";
} // namespace hestia::ipc::topics
