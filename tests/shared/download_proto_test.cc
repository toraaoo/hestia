#include <gtest/gtest.h>

#include <string>

#include <hestia/proto/download.h>

using namespace hestia::proto;

namespace {
    template <typename T>
    T round_trip(const T &value) {
        return nlohmann::json(value).get<T>();
    }
} // namespace

TEST(DownloadProto, AlgorithmRoundTrip) {
    EXPECT_EQ(parse_hash_algorithm(to_string(HashAlgorithm::sha1)), HashAlgorithm::sha1);
    EXPECT_EQ(parse_hash_algorithm(to_string(HashAlgorithm::sha256)), HashAlgorithm::sha256);
    EXPECT_EQ(parse_hash_algorithm("md5"), std::nullopt);
    EXPECT_EQ(parse_hash_algorithm(""), std::nullopt);
}

TEST(DownloadProto, SpecRoundTripWithChecksum) {
    DownloadSpec spec;
    spec.id = "dl-1";
    spec.url = "https://example.com/file.bin";
    spec.destination = "/tmp/file.bin";
    spec.checksum = Checksum{HashAlgorithm::sha256, std::string(64, 'a')};

    const DownloadSpec back = round_trip(spec);
    EXPECT_EQ(back.id, "dl-1");
    EXPECT_EQ(back.url, spec.url);
    EXPECT_EQ(back.destination, spec.destination);
    ASSERT_TRUE(back.checksum.has_value());
    EXPECT_EQ(back.checksum->algorithm, HashAlgorithm::sha256);
    EXPECT_EQ(back.checksum->hex, spec.checksum->hex);
}

TEST(DownloadProto, SpecRoundTripWithoutChecksum) {
    DownloadSpec spec;
    spec.url = "https://example.com/file.bin";
    spec.destination = "/tmp/file.bin";

    const DownloadSpec back = round_trip(spec);
    EXPECT_TRUE(back.id.empty());
    EXPECT_EQ(back.url, spec.url);
    EXPECT_EQ(back.destination, spec.destination);
    EXPECT_FALSE(back.checksum.has_value());
}

TEST(DownloadProto, SpecRejectsUnknownAlgorithm) {
    const nlohmann::json payload = {
        {"url", "https://example.com/f"},
        {"dest", "/tmp/f"},
        {"checksum", {{"algorithm", "md5"}, {"hex", std::string(32, 'a')}}},
    };
    EXPECT_THROW(payload.get<DownloadSpec>(), std::exception);
}

TEST(DownloadProto, ProgressRoundTrip) {
    DownloadProgress progress{.downloaded = 512, .total = 2048};
    const DownloadProgress back = round_trip(progress);
    EXPECT_EQ(back.downloaded, 512u);
    EXPECT_EQ(back.total, 2048u);
}

TEST(DownloadProto, ProgressEventFlattensWithId) {
    const nlohmann::json j =
        DownloadProgressEvent{.id = "dl-1", .progress = {.downloaded = 10, .total = 20}};
    EXPECT_EQ(j.at("id"), "dl-1");
    EXPECT_EQ(j.at("downloaded"), 10u);

    const auto back = j.get<DownloadProgressEvent>();
    EXPECT_EQ(back.id, "dl-1");
    EXPECT_EQ(back.progress.total, 20u);
}

TEST(DownloadProto, ValidityHelper) {
    EXPECT_TRUE(is_valid_checksum(Checksum{HashAlgorithm::sha1, std::string(40, 'a')}));
    EXPECT_TRUE(is_valid_checksum(Checksum{HashAlgorithm::sha256, std::string(64, 'f')}));

    // Wrong length for the algorithm.
    EXPECT_FALSE(is_valid_checksum(Checksum{HashAlgorithm::sha256, std::string(40, 'a')}));
    EXPECT_FALSE(is_valid_checksum(Checksum{HashAlgorithm::sha1, "abc123"}));

    // Right length, non-hex characters.
    EXPECT_FALSE(is_valid_checksum(Checksum{HashAlgorithm::sha1, std::string(40, 'z')}));
}
