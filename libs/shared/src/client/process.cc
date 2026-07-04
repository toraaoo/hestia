#include "hestia/client/process.h"

#include <utility>

#include <hestia/proto/events.h>

#include "session.h"

namespace hestia::client {
    proto::ProcessRecord Process::start(const proto::LaunchSpec &spec) {
        return session_->call<proto::ProcessStart>(spec);
    }

    void Process::stop(std::string_view id) {
        session_->call<proto::ProcessStop>({.id = std::string(id)});
    }

    std::vector<proto::ProcessRecord> Process::list() {
        return session_->call<proto::ProcessList>().processes;
    }

    std::optional<proto::ProcessRecord> Process::status(std::string_view id) {
        return session_->try_call<proto::ProcessStatus>({.id = std::string(id)});
    }

    std::string Process::logs(std::string_view id, int lines) {
        return session_->call<proto::ProcessLogs>({.id = std::string(id), .lines = lines}).text;
    }

    void Process::subscribe(ProcessEventCallback cb, const std::string &id_filter) {
        session_->set_event_callback([cb = std::move(cb)](const ipc::Event &event) {
            if (!event.topic.starts_with("process.")) return;
            ProcessEvent out;
            out.topic = event.topic;
            out.id = event.payload.value("id", std::string{});
            if (event.topic == proto::ProcessLogEvent::kTopic) {
                out.log = event.payload.get<proto::ProcessLogEvent>().text;
            } else {
                out.record = event.payload.get<proto::ProcessRecord>();
            }
            cb(out);
        });
        session_->call<proto::EventsSubscribe>({.id = id_filter});
    }
} // namespace hestia::client
