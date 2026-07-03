#include <gtest/gtest.h>

#include <filesystem>
#include <fstream>
#include <stdexcept>
#include <string>

#include <hestia/engine/cache.h>
#include <hestia/engine/downloader.h>

#include "download/checksum.h"

using namespace hestia::engine;
using hestia::ipc::Checksum;
using hestia::ipc::HashAlgorithm;
namespace fs = std::filesystem;

namespace {
    Checksum sha256_of(const std::string &content) {
        Hasher hasher(HashAlgorithm::sha256);
        hasher.update(content.data(), content.size());
        return Checksum{.algorithm = HashAlgorithm::sha256, .hex = hasher.hex_digest()};
    }

    fs::path write_file(const fs::path &path, const std::string &content) {
        fs::create_directories(path.parent_path());
        std::ofstream(path, std::ios::binary) << content;
        return path;
    }

    std::string read_file(const fs::path &path) {
        std::ifstream in(path, std::ios::binary);
        return {std::istreambuf_iterator<char>(in), std::istreambuf_iterator<char>()};
    }
} // namespace

TEST(Cache, StoreAndLookupRoundTrip) {
    const fs::path base = fs::temp_directory_path() / "hestia_test_cache_roundtrip";
    fs::remove_all(base);
    Cache cache{base / "cache"};

    const std::string content = "mod jar bytes";
    const auto checksum = sha256_of(content);
    EXPECT_FALSE(cache.lookup(checksum).has_value());

    cache.store(write_file(base / "downloaded.jar", content), checksum);
    const auto blob = cache.lookup(checksum);
    ASSERT_TRUE(blob.has_value());
    EXPECT_EQ(read_file(*blob), content);

    const auto usage = cache.usage();
    EXPECT_EQ(usage.entries, 1U);
    EXPECT_EQ(usage.bytes, content.size());
    fs::remove_all(base);
}

TEST(Cache, EvictRemovesTheBlob) {
    const fs::path base = fs::temp_directory_path() / "hestia_test_cache_evict";
    fs::remove_all(base);
    Cache cache{base / "cache"};
    const auto checksum = sha256_of("bytes");
    cache.store(write_file(base / "f", "bytes"), checksum);
    ASSERT_TRUE(cache.lookup(checksum).has_value());
    cache.evict(checksum);
    EXPECT_FALSE(cache.lookup(checksum).has_value());
    fs::remove_all(base);
}

TEST(Cache, ClearReportsWhatItFreed) {
    const fs::path base = fs::temp_directory_path() / "hestia_test_cache_clear";
    fs::remove_all(base);
    Cache cache{base / "cache"};
    cache.store(write_file(base / "a", "aaaa"), sha256_of("aaaa"));
    cache.store(write_file(base / "b", "bb"), sha256_of("bb"));

    const auto freed = cache.clear();
    EXPECT_EQ(freed.entries, 2U);
    EXPECT_EQ(freed.bytes, 6U);
    EXPECT_EQ(cache.usage().entries, 0U);
    fs::remove_all(base);
}

TEST(Cache, IgnoresInvalidChecksums) {
    Cache cache{fs::temp_directory_path() / "hestia_test_cache_invalid"};
    const Checksum bad{.algorithm = HashAlgorithm::sha256, .hex = "not-hex"};
    EXPECT_FALSE(cache.lookup(bad).has_value());
}

TEST(Downloader, ServesFromCacheWithoutNetwork) {
    const fs::path base = fs::temp_directory_path() / "hestia_test_cache_hit";
    fs::remove_all(base);
    Cache cache{base / "cache"};

    const std::string content = "cached archive";
    const auto checksum = sha256_of(content);
    cache.store(write_file(base / "seed", content), checksum);

    const fs::path dest = base / "out" / "archive";
    bool progressed = false;
    Downloader{&cache}.fetch("http://localhost:1/unreachable", dest, checksum,
                             [&](const hestia::ipc::DownloadProgress &p) {
                                 progressed = true;
                                 EXPECT_EQ(p.total, content.size());
                             });

    EXPECT_EQ(read_file(dest), content);
    EXPECT_TRUE(progressed);
    fs::remove_all(base);
}

TEST(Downloader, EvictsCorruptBlobAndFallsBackToNetwork) {
    const fs::path base = fs::temp_directory_path() / "hestia_test_cache_corrupt";
    fs::remove_all(base);
    Cache cache{base / "cache"};

    const auto checksum = sha256_of("original");
    cache.store(write_file(base / "seed", "original"), checksum);
    std::ofstream(*cache.lookup(checksum), std::ios::binary | std::ios::trunc) << "tampered";

    const fs::path dest = base / "out" / "archive";
    EXPECT_THROW(Downloader{&cache}.fetch("http://localhost:1/unreachable", dest, checksum), std::runtime_error);
    EXPECT_FALSE(cache.lookup(checksum).has_value());
    EXPECT_FALSE(fs::exists(dest));
    fs::remove_all(base);
}
