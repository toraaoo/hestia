#include <gtest/gtest.h>

#include <filesystem>
#include <stdexcept>

#include <hestia/engine/downloader.h>
#include <hestia/proto/download.h>

using hestia::engine::Downloader;
using hestia::proto::Checksum;
using hestia::proto::HashAlgorithm;

namespace fs = std::filesystem;

TEST(Downloader, RejectsEmptyUrl) {
    EXPECT_THROW(Downloader{}.fetch("", fs::temp_directory_path() / "hestia-dl-test"), std::runtime_error);
}

// Hex validation happens before any network or file I/O, so no server is needed
// and no .part file may appear.
TEST(Downloader, RejectsChecksumHexOfWrongLength) {
    const fs::path dest = fs::temp_directory_path() / "hestia-dl-test-bad-hex";
    const Checksum checksum{.algorithm = HashAlgorithm::sha256, .hex = "abc123"};
    EXPECT_THROW(Downloader{}.fetch("http://localhost/never-contacted", dest, checksum), std::runtime_error);
    EXPECT_FALSE(fs::exists(dest));
    EXPECT_FALSE(fs::exists(dest.string() + ".part"));
}

TEST(Downloader, RejectsChecksumWithNonHexCharacters) {
    const fs::path dest = fs::temp_directory_path() / "hestia-dl-test-non-hex";
    const Checksum checksum{.algorithm = HashAlgorithm::sha1, .hex = std::string(40, 'z')};
    EXPECT_THROW(Downloader{}.fetch("http://localhost/never-contacted", dest, checksum), std::runtime_error);
    EXPECT_FALSE(fs::exists(dest));
    EXPECT_FALSE(fs::exists(dest.string() + ".part"));
}
