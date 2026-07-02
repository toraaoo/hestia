#include <gtest/gtest.h>

#include <string>

#include <hestia/ipc/download_codec.h>

using namespace hestia::ipc;

TEST(DownloadCodec, AlgorithmRoundTrip) {
    EXPECT_EQ(parse_hash_algorithm(to_string(HashAlgorithm::sha1)), HashAlgorithm::sha1);
    EXPECT_EQ(parse_hash_algorithm(to_string(HashAlgorithm::sha256)), HashAlgorithm::sha256);
    EXPECT_EQ(parse_hash_algorithm("md5"), std::nullopt);
    EXPECT_EQ(parse_hash_algorithm(""), std::nullopt);
}

TEST(DownloadCodec, SpecRoundTripWithChecksum) {
    DownloadSpec spec;
    spec.id = "dl-1";
    spec.url = "https://example.com/file.bin";
    spec.destination = "/tmp/file.bin";
    spec.checksum = Checksum{HashAlgorithm::sha256, std::string(64, 'a')};

    const DownloadSpec back = download_spec_from_json(to_json(spec));
    EXPECT_EQ(back.id, "dl-1");
    EXPECT_EQ(back.url, spec.url);
    EXPECT_EQ(back.destination, spec.destination);
    ASSERT_TRUE(back.checksum.has_value());
    EXPECT_EQ(back.checksum->algorithm, HashAlgorithm::sha256);
    EXPECT_EQ(back.checksum->hex, spec.checksum->hex);
}

TEST(DownloadCodec, SpecRoundTripWithoutChecksum) {
    DownloadSpec spec;
    spec.url = "https://example.com/file.bin";
    spec.destination = "/tmp/file.bin";

    const DownloadSpec back = download_spec_from_json(to_json(spec));
    EXPECT_TRUE(back.id.empty());
    EXPECT_EQ(back.url, spec.url);
    EXPECT_EQ(back.destination, spec.destination);
    EXPECT_FALSE(back.checksum.has_value());
}

TEST(DownloadCodec, SpecRejectsUnknownAlgorithm) {
    nlohmann::json payload = {
        {"url", "https://example.com/f"},
        {"dest", "/tmp/f"},
        {"checksum", {{"algorithm", "md5"}, {"hex", std::string(32, 'a')}}},
    };
    EXPECT_THROW(download_spec_from_json(payload), std::exception);
}

TEST(DownloadCodec, ProgressRoundTrip) {
    DownloadProgress progress{.downloaded = 512, .total = 2048};
    const DownloadProgress back = progress_from_json(to_json(progress));
    EXPECT_EQ(back.downloaded, 512u);
    EXPECT_EQ(back.total, 2048u);
}

TEST(DownloadCodec, ValidityHelper) {
    EXPECT_TRUE(is_valid_checksum(Checksum{HashAlgorithm::sha1, std::string(40, 'a')}));
    EXPECT_TRUE(is_valid_checksum(Checksum{HashAlgorithm::sha256, std::string(64, 'f')}));

    // Wrong length for the algorithm.
    EXPECT_FALSE(is_valid_checksum(Checksum{HashAlgorithm::sha256, std::string(40, 'a')}));
    EXPECT_FALSE(is_valid_checksum(Checksum{HashAlgorithm::sha1, "abc123"}));

    // Right length, non-hex characters.
    EXPECT_FALSE(is_valid_checksum(Checksum{HashAlgorithm::sha1, std::string(40, 'z')}));
}
