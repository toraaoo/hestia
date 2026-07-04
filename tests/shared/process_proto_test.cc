#include <gtest/gtest.h>

#include <chrono>

#include <hestia/proto/process.h>

using namespace hestia::proto;

namespace {
    template <typename T>
    T round_trip(const T &value) {
        return nlohmann::json(value).get<T>();
    }
} // namespace

TEST(ProcessProto, KindAndStateRoundTrip) {
    EXPECT_EQ(parse_kind(to_string(ProcessKind::Server)), ProcessKind::Server);
    EXPECT_EQ(parse_kind(to_string(ProcessKind::Instance)), ProcessKind::Instance);
    EXPECT_EQ(parse_kind("nonsense"), ProcessKind::Server); // unknown defaults to server

    for (const auto state : {ProcessState::Starting, ProcessState::Running,
                            ProcessState::Exited, ProcessState::Crashed}) {
        EXPECT_EQ(parse_state(to_string(state)), state);
    }
    EXPECT_EQ(parse_state("nonsense"), ProcessState::Starting); // unknown defaults to starting
}

TEST(ProcessProto, RestartPolicyRoundTrip) {
    RestartPolicy policy;
    policy.auto_restart = true;
    policy.max_retries = 7;
    policy.backoff = std::chrono::milliseconds(2500);

    const RestartPolicy back = round_trip(policy);
    EXPECT_TRUE(back.auto_restart);
    EXPECT_EQ(back.max_retries, 7);
    EXPECT_EQ(back.backoff, std::chrono::milliseconds(2500));
}

TEST(ProcessProto, RecordRoundTrip) {
    ProcessRecord rec;
    rec.id = "srv";
    rec.kind = ProcessKind::Instance;
    rec.pid = 4321;
    rec.start_time = 1010;
    rec.log_path = "/var/log/srv.log";
    rec.state = ProcessState::Crashed;
    rec.program = "/bin/srv";
    rec.args = {"--port", "25565"};
    rec.working_dir = "/srv";
    rec.restart.auto_restart = true;
    rec.restart.max_retries = 3;
    rec.restarts = 1;

    const ProcessRecord back = round_trip(rec);
    EXPECT_EQ(back.id, "srv");
    EXPECT_EQ(back.kind, ProcessKind::Instance);
    EXPECT_EQ(back.pid, 4321);
    EXPECT_EQ(back.start_time, 1010);
    EXPECT_EQ(back.state, ProcessState::Crashed);
    EXPECT_EQ(back.program, rec.program);
    EXPECT_EQ(back.log_path, rec.log_path);
    ASSERT_EQ(back.args.size(), 2u);
    EXPECT_EQ(back.args[1], "25565");
    EXPECT_EQ(back.working_dir, rec.working_dir);
    EXPECT_EQ(back.restart.max_retries, 3);
    EXPECT_EQ(back.restarts, 1);
}

TEST(ProcessProto, LaunchSpecRoundTrip) {
    LaunchSpec spec;
    spec.id = "mc";
    spec.kind = ProcessKind::Server;
    spec.program = "/bin/java";
    spec.args = {"-jar", "server.jar"};
    spec.working_dir = "/world";
    spec.restart.auto_restart = true;
    spec.restart.max_retries = 2;

    const LaunchSpec back = round_trip(spec);
    EXPECT_EQ(back.id, "mc");
    EXPECT_EQ(back.program, spec.program);
    ASSERT_EQ(back.args.size(), 2u);
    EXPECT_EQ(back.working_dir, spec.working_dir);
    EXPECT_TRUE(back.restart.auto_restart);
    EXPECT_EQ(back.restart.max_retries, 2);
}

TEST(ProcessProto, StateEventCarriesRecordFlat) {
    ProcessRecord rec;
    rec.id = "srv";
    rec.state = ProcessState::Running;

    const nlohmann::json j = ProcessStateEvent{.record = rec};
    // The event's payload IS the record (id at the top level), which is what
    // the hub's id-filtering matches against.
    EXPECT_EQ(j.at("id"), "srv");
    EXPECT_EQ(j.at("state"), "running");

    const auto back = j.get<ProcessStateEvent>();
    EXPECT_EQ(back.record.id, "srv");
    EXPECT_EQ(back.record.state, ProcessState::Running);
}
