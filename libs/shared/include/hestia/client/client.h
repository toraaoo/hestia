#pragma once

#include <cstdint>
#include <filesystem>
#include <functional>
#include <memory>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

#include <hestia/ipc/protocol.h>

// The thin client SDK every frontend (CLI/TUI/desktop/tray) uses to drive the
// daemon — the single boundary they code against. One persistent, multiplexed
// connection: typed calls correlated by id, plus pushed events to a subscriber.
namespace hestia::client {
    struct AppInfo {
        std::string name;
        std::string version;
        std::string id;
        std::string vendor;
        std::string channel;
    };

    // What the daemon should do when a process exits unexpectedly. `max_retries`
    // of 0 means restart without limit.
    struct RestartPolicy {
        bool auto_restart = false;
        int max_retries = 0;
        long long backoff_ms = 1000;
    };

    // A process to launch via the daemon. `kind` is "server" or "instance".
    struct ProcessSpec {
        std::string id;
        std::string kind = "server";
        std::string program;
        std::vector<std::string> args;
        std::string cwd;
        RestartPolicy restart;
    };

    // A supervised process as reported by the daemon.
    struct ProcessInfo {
        std::string id;
        std::string kind;
        std::string state;
        long long pid = 0;
        long long start_time = 0;
        std::string log_path;
    };

    // An event pushed by the daemon to a subscriber. A "process.state" event
    // carries `process`; a "process.log" event carries `log` (a chunk of new
    // output). `id` is the process the event concerns.
    struct ProcessEvent {
        std::string topic;
        std::string id;
        std::optional<ProcessInfo> process;
        std::optional<std::string> log;
    };

    using EventCallback = std::function<void(const ProcessEvent &)>;

    // A file to download via the daemon. Empty checksum fields mean no
    // verification; `checksum_algorithm` is "sha1" or "sha256".
    struct DownloadRequest {
        std::string url;
        std::string destination; // absolute path
        std::string checksum_algorithm;
        std::string checksum_hex;
    };

    struct DownloadProgress {
        std::uint64_t downloaded = 0;
        std::uint64_t total = 0; // 0 = unknown
    };

    using DownloadProgressCallback = std::function<void(const DownloadProgress &)>;

    struct JavaRelease {
        int major = 0;
        bool lts = false;
    };

    struct JavaRuntime {
        std::string vendor;
        int major = 0;
        std::string release_name;
        std::string home;
        std::string executable;
    };

    struct JavaInstallProgress {
        std::string phase;         // "resolving" | "downloading" | "extracting"
        std::uint64_t current = 0; // bytes while downloading, entries while extracting
        std::uint64_t total = 0;   // 0 = unknown
    };

    using JavaInstallProgressCallback = std::function<void(const JavaInstallProgress &)>;

    struct CacheEntry {
        std::string algorithm;
        std::string hex;
        std::uint64_t size = 0;
    };

    struct CacheStats {
        std::string path;
        std::uint64_t entries = 0;
        std::uint64_t bytes = 0;
    };

    struct DaemonStatus {
        long long pid = 0;
        std::string version;
        long long uptime_seconds = 0;
        std::string home;
        std::string log;
    };

    class Client {
    public:
        // Connect to the running daemon. If none is running and `auto_spawn` is
        // true, start one and wait for it to come up. Throws std::runtime_error
        // if the daemon is unreachable (and could not be spawned).
        static Client connect(bool auto_spawn = true);

        Client(Client &&) noexcept;
        Client &operator=(Client &&) noexcept;
        ~Client();

        // Raw request; throws only on transport failure (a daemon-side error is a
        // Response with ok == false). The typed channels below are built on this.
        ipc::Response call(const std::string &channel, const nlohmann::json &payload);

        // Typed channels. These throw std::runtime_error on a transport failure
        // or a daemon-side error (except config_get, which returns nullopt for a
        // missing key).
        std::optional<std::string> config_get(std::string_view key);
        void config_set(std::string_view key, std::string_view value);
        std::filesystem::path config_home();
        std::filesystem::path config_set_home(std::string_view dir);
        AppInfo app_info();

        // Autostart: register/unregister the daemon to start with the user
        // session, and query the current state. Backed by the platform's native
        // mechanism (systemd user unit / LaunchAgent / logon Scheduled Task).
        void autostart_enable();
        void autostart_disable();
        bool autostart_status();

        // Process supervision. start/stop/list/status/logs round-trip to the
        // daemon, which owns the processes so they outlive this client.
        ProcessInfo process_start(const ProcessSpec &spec);
        void process_stop(std::string_view id);
        std::vector<ProcessInfo> process_list();
        std::optional<ProcessInfo> process_status(std::string_view id);
        std::string process_logs(std::string_view id, int lines = 200);

        // Stream live process events. `cb` is invoked on the connection's reader
        // thread for each matching event; pass an `id_filter` to scope it to one
        // process, or empty for all. Call before issuing further requests; the
        // history up to now is available via process_logs.
        void subscribe(EventCallback cb, const std::string &id_filter = {});

        // Download a file via the daemon, blocking until it completes;
        // `on_progress` is invoked on the reader thread as bytes arrive. Throws
        // std::runtime_error on failure (bad request, network error, checksum
        // mismatch). Uses the client's single event-callback slot, so it
        // replaces any callback installed by subscribe().
        void download(const DownloadRequest &request, const DownloadProgressCallback &on_progress = {});

        // Java runtimes, managed by the daemon. java_install blocks until the
        // runtime is registered, reporting progress on the reader thread; like
        // download(), it uses the client's single event-callback slot.
        std::vector<JavaRelease> java_releases();
        std::vector<JavaRuntime> java_list();
        JavaRuntime java_install(int major, const JavaInstallProgressCallback &on_progress = {});
        void java_uninstall(int major);

        // The daemon's content-addressed download cache. cache_clear reports
        // what was removed (its path field is the cache location).
        CacheStats cache_info();
        std::vector<CacheEntry> cache_list();
        CacheStats cache_clear();

        // Daemon lifecycle. daemon_stop asks the daemon to shut itself down;
        // it answers before exiting, so poll the endpoint to observe the exit.
        DaemonStatus daemon_status();
        void daemon_stop();

    private:
        struct Detail;
        explicit Client(std::unique_ptr<Detail> detail);

        nlohmann::json run_job(const std::string &id, const char *done_topic, const char *error_topic,
                               const std::function<void(const ipc::Event &)> &on_event,
                               const std::function<void()> &start);

        std::unique_ptr<Detail> d_;
    };
} // namespace hestia::client
