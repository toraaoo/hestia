#include <gtest/gtest.h>

#include <filesystem>
#include <fstream>
#include <stdexcept>

#include <cstdlib>

#include <hestia/engine/java.h>

#include "java/adoptium_provider.h"
#include "java/extract.h"
#include "java/platform.h"

using namespace hestia::engine;
namespace fs = std::filesystem;
using nlohmann::json;

namespace {
    const json kAssets = json::array({
        {
            {"release_name", "jdk-21.0.7+6"},
            {"binary",
             {
                 {"os", "linux"},
                 {"architecture", "x64"},
                 {"image_type", "jdk"},
                 {"package",
                  {
                      {"name", "OpenJDK21U-jdk_x64_linux_hotspot_21.0.7_6.tar.gz"},
                      {"link", "https://example.com/jdk21.tar.gz"},
                      {"checksum", std::string(64, 'a')},
                  }},
             }},
        },
        {
            {"release_name", "jdk-21.0.7+6"},
            {"binary",
             {
                 {"os", "windows"},
                 {"architecture", "x64"},
                 {"image_type", "jdk"},
                 {"package",
                  {
                      {"name", "OpenJDK21U-jdk_x64_windows_hotspot_21.0.7_6.zip"},
                      {"link", "https://example.com/jdk21.zip"},
                      {"checksum", std::string(64, 'b')},
                  }},
             }},
        },
    });

    fs::path make_fake_install(const fs::path &base, const std::string &vendor, int major,
                               const std::string &release_name) {
        const fs::path install_dir = base / (vendor + "-" + std::to_string(major));
        const fs::path bin = install_dir / release_name / "bin";
        fs::create_directories(bin);
#if defined(_WIN32)
        const fs::path exe = bin / "java.exe";
#else
        const fs::path exe = bin / "java";
#endif
        std::ofstream(exe) << "";
        std::ofstream(install_dir / "runtime.json") << json{
            {"vendor", vendor},
            {"major", major},
            {"release_name", release_name},
            {"executable", (fs::path(release_name) / "bin" / exe.filename()).generic_string()},
        }.dump();
        return install_dir;
    }
} // namespace

TEST(AdoptiumProvider, ParsesReleaseLines) {
    const json j{
        {"available_releases", {8, 11, 17, 21, 24}},
        {"available_lts_releases", {8, 11, 17, 21}},
    };
    const auto releases = adoptium_releases_from_json(j);
    ASSERT_EQ(releases.size(), 5U);
    EXPECT_EQ(releases.front().major, 8);
    EXPECT_TRUE(releases.front().lts);
    EXPECT_EQ(releases.back().major, 24);
    EXPECT_FALSE(releases.back().lts);
}

TEST(AdoptiumProvider, RejectsResponseWithoutReleases) {
    EXPECT_THROW(adoptium_releases_from_json(json::object()), std::runtime_error);
}

TEST(AdoptiumProvider, ResolvesPackageForTarget) {
    const auto package = adoptium_package_from_json(kAssets, 21, JavaTarget{.os = "linux", .arch = "x64"});
    EXPECT_EQ(package.vendor, "temurin");
    EXPECT_EQ(package.major, 21);
    EXPECT_EQ(package.release_name, "jdk-21.0.7+6");
    EXPECT_EQ(package.url, "https://example.com/jdk21.tar.gz");
    EXPECT_EQ(package.archive_name, "OpenJDK21U-jdk_x64_linux_hotspot_21.0.7_6.tar.gz");
    EXPECT_EQ(package.checksum.hex, std::string(64, 'a'));

    const auto windows = adoptium_package_from_json(kAssets, 21, JavaTarget{.os = "windows", .arch = "x64"});
    EXPECT_EQ(windows.archive_name, "OpenJDK21U-jdk_x64_windows_hotspot_21.0.7_6.zip");
}

TEST(AdoptiumProvider, ThrowsWhenNoBuildMatchesTarget) {
    EXPECT_THROW(adoptium_package_from_json(kAssets, 21, JavaTarget{.os = "mac", .arch = "aarch64"}),
                 std::runtime_error);
}

TEST(AdoptiumProvider, ThrowsOnMissingChecksum) {
    auto assets = kAssets;
    assets[0]["binary"]["package"].erase("checksum");
    EXPECT_THROW(adoptium_package_from_json(assets, 21, JavaTarget{.os = "linux", .arch = "x64"}),
                 std::runtime_error);
}

TEST(JavaPlatform, HostTargetUsesAdoptiumVocabulary) {
    const auto target = host_target();
    EXPECT_TRUE(target.os == "linux" || target.os == "mac" || target.os == "windows");
    EXPECT_TRUE(target.arch == "x64" || target.arch == "aarch64");
}

TEST(JavaPlatform, FindsExecutableUnderNestedArchiveRoot) {
    const fs::path root = fs::temp_directory_path() / "hestia_test_java_find";
    fs::remove_all(root);
#if defined(_WIN32)
    const fs::path exe = root / "jdk-21.0.7+6" / "bin" / "java.exe";
#else
    const fs::path exe = root / "jdk-21.0.7+6" / "bin" / "java";
#endif
    fs::create_directories(exe.parent_path());
    std::ofstream(exe) << "";
    EXPECT_EQ(find_java_executable(root), exe);
    fs::remove_all(root);
}

TEST(JavaPlatform, ReturnsNulloptWhenNoExecutable) {
    const fs::path root = fs::temp_directory_path() / "hestia_test_java_find_none";
    fs::remove_all(root);
    fs::create_directories(root / "docs");
    EXPECT_FALSE(find_java_executable(root).has_value());
    fs::remove_all(root);
}

TEST(Java, RequiresAProvider) {
    EXPECT_THROW(Java(fs::temp_directory_path(), std::vector<std::unique_ptr<JavaProvider>>{}),
                 std::invalid_argument);
}

TEST(Java, ListsInstalledRuntimesFromDisk) {
    const fs::path base = fs::temp_directory_path() / "hestia_test_java_list";
    fs::remove_all(base);
    make_fake_install(base, "temurin", 17, "jdk-17.0.11+9");
    make_fake_install(base, "temurin", 21, "jdk-21.0.7+6");
    fs::create_directories(base / "tmp");

    const Java java{base};
    const auto runtimes = java.installed();
    ASSERT_EQ(runtimes.size(), 2U);
    EXPECT_EQ(runtimes[0].major, 17);
    EXPECT_EQ(runtimes[1].major, 21);
    EXPECT_EQ(runtimes[1].vendor, "temurin");
    EXPECT_EQ(runtimes[1].release_name, "jdk-21.0.7+6");
    EXPECT_TRUE(fs::exists(runtimes[1].executable));
    EXPECT_EQ(runtimes[1].home.filename(), "jdk-21.0.7+6");
    fs::remove_all(base);
}

TEST(Java, SkipsDirectoriesWithoutAValidRecord) {
    const fs::path base = fs::temp_directory_path() / "hestia_test_java_skip";
    fs::remove_all(base);
    fs::create_directories(base / "temurin-21");
    std::ofstream(base / "temurin-21" / "runtime.json") << "not json";
    const Java java{base};
    EXPECT_TRUE(java.installed().empty());
    fs::remove_all(base);
}

TEST(Java, UninstallRemovesTheInstallDirectory) {
    const fs::path base = fs::temp_directory_path() / "hestia_test_java_uninstall";
    fs::remove_all(base);
    const auto install_dir = make_fake_install(base, "temurin", 21, "jdk-21.0.7+6");

    Java java{base};
    EXPECT_TRUE(java.uninstall(21));
    EXPECT_FALSE(fs::exists(install_dir));
    EXPECT_FALSE(java.uninstall(21));
    fs::remove_all(base);
}

TEST(ExtractArchive, ExtractsAndReportsPerEntryProgress) {
    const fs::path base = fs::temp_directory_path() / "hestia_test_extract";
    fs::remove_all(base);
    fs::create_directories(base / "src" / "bin");
    std::ofstream(base / "src" / "bin" / "java") << "binary";
    std::ofstream(base / "src" / "release") << "JAVA_VERSION=21";

    const fs::path archive = base / "jdk.tar.gz";
    const std::string cmd =
        "tar -czf \"" + archive.string() + "\" -C \"" + (base / "src").string() + "\" .";
    ASSERT_EQ(std::system(cmd.c_str()), 0);

    std::uint64_t last_done = 0;
    std::uint64_t last_total = 0;
    extract_archive(archive, base / "out", [&](std::uint64_t done, std::uint64_t total) {
        last_done = done;
        last_total = total;
    });

    EXPECT_GT(last_done, 0U);
    EXPECT_EQ(last_done, last_total);
    EXPECT_TRUE(fs::exists(base / "out" / "bin" / "java"));
    EXPECT_TRUE(fs::exists(base / "out" / "release"));
    fs::remove_all(base);
}

TEST(ExtractArchive, ThrowsOnMissingArchive) {
    const fs::path base = fs::temp_directory_path() / "hestia_test_extract_missing";
    fs::remove_all(base);
    EXPECT_THROW(extract_archive(base / "nope.tar.gz", base / "out"), std::runtime_error);
    fs::remove_all(base);
}

TEST(Java, RejectsNonPositiveMajor) {
    Java java{fs::temp_directory_path() / "hestia_test_java_invalid"};
    EXPECT_THROW(java.install(0), std::runtime_error);
    EXPECT_THROW(java.install(-3), std::runtime_error);
}
