#include <gtest/gtest.h>

#include <hestia/ipc/java_codec.h>

using namespace hestia::ipc;

TEST(JavaCodec, RuntimeRoundTrips) {
    JavaRuntime runtime;
    runtime.vendor = "temurin";
    runtime.major = 21;
    runtime.release_name = "jdk-21.0.7+6";
    runtime.home = "/data/java/temurin-21/jdk-21.0.7+6";
    runtime.executable = "/data/java/temurin-21/jdk-21.0.7+6/bin/java";

    const auto decoded = java_runtime_from_json(to_json(runtime));
    EXPECT_EQ(decoded.vendor, runtime.vendor);
    EXPECT_EQ(decoded.major, runtime.major);
    EXPECT_EQ(decoded.release_name, runtime.release_name);
    EXPECT_EQ(decoded.home, runtime.home);
    EXPECT_EQ(decoded.executable, runtime.executable);
}

TEST(JavaCodec, ReleaseRoundTrips) {
    const auto decoded = java_release_from_json(to_json(JavaRelease{.major = 21, .lts = true}));
    EXPECT_EQ(decoded.major, 21);
    EXPECT_TRUE(decoded.lts);
}

TEST(JavaCodec, ProgressRoundTrips) {
    const JavaInstallProgress progress{.phase = JavaInstallPhase::downloading, .current = 1024, .total = 4096};
    const auto decoded = java_install_progress_from_json(to_json(progress));
    EXPECT_EQ(decoded.phase, JavaInstallPhase::downloading);
    EXPECT_EQ(decoded.current, 1024U);
    EXPECT_EQ(decoded.total, 4096U);
}

TEST(JavaCodec, PhaseNamesRoundTrip) {
    for (const auto phase:
         {JavaInstallPhase::resolving, JavaInstallPhase::downloading, JavaInstallPhase::extracting}) {
        EXPECT_EQ(parse_java_install_phase(to_string(phase)), phase);
    }
    EXPECT_FALSE(parse_java_install_phase("verifying").has_value());
}
