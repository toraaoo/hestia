#pragma once

#include <chrono>
#include <cstdint>
#include <filesystem>
#include <string>
#include <string_view>
#include <vector>

#include <hestia/proto/contract.h>

// The process-supervision domain: types shared by the daemon (which owns the
// processes) and the client SDK (which reports them), plus the contracts for
// the process.* channels and events.
namespace hestia::proto {
    // What kind of process this is — drives the default restart policy (servers
    // may auto-restart; client instances do not).
    enum class ProcessKind : std::uint8_t { Server, Instance };

    enum class ProcessState : std::uint8_t { Starting, Running, Exited, Crashed };

    const char *to_string(ProcessKind kind);
    const char *to_string(ProcessState state);
    ProcessKind parse_kind(std::string_view kind);
    ProcessState parse_state(std::string_view state);

    void to_json(nlohmann::json &j, ProcessKind kind);
    void from_json(const nlohmann::json &j, ProcessKind &kind);
    void to_json(nlohmann::json &j, ProcessState state);
    void from_json(const nlohmann::json &j, ProcessState &state);

    // What to do when a supervised process exits unexpectedly. `max_retries` of 0
    // means restart without limit; a positive value caps the attempts.
    struct RestartPolicy {
        bool auto_restart = false;
        int max_retries = 0;
        std::chrono::milliseconds backoff{1000};

        static constexpr auto kFields =
            fields(field("auto", &RestartPolicy::auto_restart), field("max_retries", &RestartPolicy::max_retries),
                   field("backoff_ms", &RestartPolicy::backoff));
    };

    // A request to launch a process.
    struct LaunchSpec {
        std::string id; // caller-assigned, stable across restarts
        ProcessKind kind = ProcessKind::Server;
        std::filesystem::path program;
        std::vector<std::string> args;
        std::filesystem::path working_dir;
        RestartPolicy restart{};

        static constexpr auto kFields =
            fields(field("id", &LaunchSpec::id, kRequired), field("kind", &LaunchSpec::kind),
                   field("program", &LaunchSpec::program, kRequired), field("args", &LaunchSpec::args),
                   field("cwd", &LaunchSpec::working_dir, kOmitIfEmpty), field("restart", &LaunchSpec::restart));
    };

    // A row of the persisted process table. Serialized so a restarted daemon can
    // re-adopt what is still running (pid + start_time disambiguates PID reuse)
    // and relaunch what should auto-restart (the launch fields are persisted too).
    struct ProcessRecord {
        std::string id;
        ProcessKind kind = ProcessKind::Server;
        std::int64_t pid = 0;
        std::int64_t start_time = 0;
        std::filesystem::path log_path;
        ProcessState state = ProcessState::Starting;

        // Enough to relaunch the process after a crash or a daemon restart.
        std::filesystem::path program;
        std::vector<std::string> args;
        std::filesystem::path working_dir;
        RestartPolicy restart{};
        int restarts = 0;

        static constexpr auto kFields = fields(
            field("id", &ProcessRecord::id, kRequired), field("kind", &ProcessRecord::kind),
            field("pid", &ProcessRecord::pid), field("start_time", &ProcessRecord::start_time),
            field("log_path", &ProcessRecord::log_path), field("state", &ProcessRecord::state),
            field("program", &ProcessRecord::program), field("args", &ProcessRecord::args),
            field("cwd", &ProcessRecord::working_dir), field("restart", &ProcessRecord::restart),
            field("restarts", &ProcessRecord::restarts));
    };

    struct ProcessId {
        std::string id;

        static constexpr auto kFields = fields(field("id", &ProcessId::id, kRequired));
    };

    struct ProcessStart {
        static constexpr const char *kChannel = "process.start";
        using Params = LaunchSpec;
        using Result = ProcessRecord;
    };

    struct ProcessStop {
        static constexpr const char *kChannel = "process.stop";
        using Params = ProcessId;
        using Result = Empty;
    };

    struct ProcessList {
        static constexpr const char *kChannel = "process.list";
        using Params = Empty;
        struct Result {
            std::vector<ProcessRecord> processes;

            static constexpr auto kFields = fields(field("processes", &Result::processes));
        };
    };

    struct ProcessStatus {
        static constexpr const char *kChannel = "process.status";
        using Params = ProcessId;
        using Result = ProcessRecord;
    };

    struct ProcessLogs {
        static constexpr const char *kChannel = "process.logs";
        struct Params {
            std::string id;
            int lines = 200;

            static constexpr auto kFields =
                fields(field("id", &Params::id, kRequired), field("lines", &Params::lines));
        };
        struct Result {
            std::string text;

            static constexpr auto kFields = fields(field("text", &Result::text));
        };
    };

    struct ProcessStateEvent {
        static constexpr const char *kTopic = "process.state";
        ProcessRecord record;

        // The payload IS the record: its id sits at the top level, which is what
        // the hub's id-filtering matches against.
        static constexpr auto kFields = fields(field("", &ProcessStateEvent::record, kFlatten));
    };

    struct ProcessLogEvent {
        static constexpr const char *kTopic = "process.log";
        std::string id;
        std::string text;

        static constexpr auto kFields =
            fields(field("id", &ProcessLogEvent::id), field("text", &ProcessLogEvent::text));
    };
} // namespace hestia::proto
