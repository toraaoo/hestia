#include <gtest/gtest.h>

#include <hestia/proto/java.h>

using namespace hestia::proto;

namespace {
    template <typename T>
    T round_trip(const T &value) {
        return nlohmann::json(value).get<T>();
    }
} // namespace

TEST(JavaProto, RuntimeRoundTrips) {
    JavaRuntime runtime;
    runtime.vendor = "temurin";
    runtime.major = 21;
    runtime.release_name = "jdk-21.0.7+6";
    runtime.home = "/data/java/temurin-21/jdk-21.0.7+6";
    runtime.executable = "/data/java/temurin-21/jdk-21.0.7+6/bin/java";

    const auto decoded = round_trip(runtime);
    EXPECT_EQ(decoded.vendor, runtime.vendor);
    EXPECT_EQ(decoded.major, runtime.major);
    EXPECT_EQ(decoded.release_name, runtime.release_name);
    EXPECT_EQ(decoded.home, runtime.home);
    EXPECT_EQ(decoded.executable, runtime.executable);
}

TEST(JavaProto, ReleaseRoundTrips) {
    const auto decoded = round_trip(JavaRelease{.major = 21, .lts = true});
    EXPECT_EQ(decoded.major, 21);
    EXPECT_TRUE(decoded.lts);
}

TEST(JavaProto, ProgressRoundTrips) {
    const JavaInstallProgress progress{.phase = JavaInstallPhase::downloading, .current = 1024, .total = 4096};
    const auto decoded = round_trip(progress);
    EXPECT_EQ(decoded.phase, JavaInstallPhase::downloading);
    EXPECT_EQ(decoded.current, 1024U);
    EXPECT_EQ(decoded.total, 4096U);
}

TEST(JavaProto, PhaseNamesRoundTrip) {
    for (const auto phase:
         {JavaInstallPhase::resolving, JavaInstallPhase::downloading, JavaInstallPhase::extracting}) {
        EXPECT_EQ(parse_java_install_phase(to_string(phase)), phase);
    }
    EXPECT_FALSE(parse_java_install_phase("verifying").has_value());
}

TEST(JavaProto, InstallDoneEventNestsRuntime) {
    JavaRuntime runtime;
    runtime.vendor = "temurin";
    runtime.major = 21;

    const nlohmann::json j = JavaInstallDoneEvent{.id = "java-1", .runtime = runtime};
    EXPECT_EQ(j.at("id"), "java-1");
    EXPECT_EQ(j.at("runtime").at("major"), 21);

    const auto back = j.get<JavaInstallDoneEvent>();
    EXPECT_EQ(back.id, "java-1");
    EXPECT_EQ(back.runtime.vendor, "temurin");
}
