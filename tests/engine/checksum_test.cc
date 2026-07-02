#include <gtest/gtest.h>

#include <algorithm>
#include <cstddef>
#include <string>

#include <hestia/engine/checksum.h>

using hestia::engine::HashAlgorithm;
using hestia::engine::Hasher;
using hestia::engine::hex_digest_length;
using hestia::engine::parse_hash_algorithm;

namespace {
    std::string digest(HashAlgorithm algorithm, const std::string &input) {
        Hasher hasher(algorithm);
        hasher.update(input.data(), input.size());
        return hasher.hex_digest();
    }
}

TEST(Checksum, ParsesAlgorithmNames) {
    EXPECT_EQ(parse_hash_algorithm("sha1"), HashAlgorithm::sha1);
    EXPECT_EQ(parse_hash_algorithm("sha256"), HashAlgorithm::sha256);
    EXPECT_EQ(parse_hash_algorithm("md5"), std::nullopt);
    EXPECT_EQ(parse_hash_algorithm(""), std::nullopt);
}

TEST(Checksum, HexDigestLengths) {
    EXPECT_EQ(hex_digest_length(HashAlgorithm::sha1), 40u);
    EXPECT_EQ(hex_digest_length(HashAlgorithm::sha256), 64u);
}

TEST(Checksum, EmptyInput) {
    EXPECT_EQ(digest(HashAlgorithm::sha1, ""),
              "da39a3ee5e6b4b0d3255bfef95601890afd80709");
    EXPECT_EQ(digest(HashAlgorithm::sha256, ""),
              "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
}

TEST(Checksum, Abc) {
    EXPECT_EQ(digest(HashAlgorithm::sha1, "abc"),
              "a9993e364706816aba3e25717850c26c9cd0d89d");
    EXPECT_EQ(digest(HashAlgorithm::sha256, "abc"),
              "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
}

TEST(Checksum, MillionAs) {
    const std::string input(1000000, 'a');
    EXPECT_EQ(digest(HashAlgorithm::sha1, input),
              "34aa973cd4c4daa4f61eeb2bdbad27316534016f");
    EXPECT_EQ(digest(HashAlgorithm::sha256, input),
              "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0");
}

TEST(Checksum, IncrementalUpdatesMatchOneShot) {
    const std::string input(100000, 'a');
    for (const auto algorithm : {HashAlgorithm::sha1, HashAlgorithm::sha256}) {
        Hasher hasher(algorithm);
        std::size_t offset = 0;
        std::size_t chunk = 1;
        while (offset < input.size()) {
            const std::size_t take = std::min(chunk, input.size() - offset);
            hasher.update(input.data() + offset, take);
            offset += take;
            chunk = chunk * 2 + 3; // odd, growing chunk sizes across block edges
        }
        EXPECT_EQ(hasher.hex_digest(), digest(algorithm, input));
    }
}
