#include "hestia/proto/process.h"

namespace hestia::proto {
    const char *to_string(ProcessKind kind) {
        return kind == ProcessKind::Instance ? "instance" : "server";
    }

    const char *to_string(ProcessState state) {
        switch (state) {
        case ProcessState::Starting: return "starting";
        case ProcessState::Running: return "running";
        case ProcessState::Exited: return "exited";
        case ProcessState::Crashed: return "crashed";
        }
        return "unknown";
    }

    ProcessKind parse_kind(std::string_view kind) {
        return kind == "instance" ? ProcessKind::Instance : ProcessKind::Server;
    }

    ProcessState parse_state(std::string_view state) {
        if (state == "running") return ProcessState::Running;
        if (state == "exited") return ProcessState::Exited;
        if (state == "crashed") return ProcessState::Crashed;
        return ProcessState::Starting;
    }

    void to_json(nlohmann::json &j, ProcessKind kind) {
        j = to_string(kind);
    }

    void from_json(const nlohmann::json &j, ProcessKind &kind) {
        kind = parse_kind(j.get<std::string>());
    }

    void to_json(nlohmann::json &j, ProcessState state) {
        j = to_string(state);
    }

    void from_json(const nlohmann::json &j, ProcessState &state) {
        state = parse_state(j.get<std::string>());
    }
} // namespace hestia::proto
