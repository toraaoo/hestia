#pragma once

#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <optional>
#include <string>
#include <string_view>

#include <hestia/proto/contract.h>

// The download domain: types shared by the daemon (which runs downloads) and
// the client SDK (which requests them), plus the contracts for the download.*
// channel and events.
namespace hestia::proto {
    enum class HashAlgorithm : std::uint8_t { sha1, sha256 };

    // "sha1" / "sha256" → the algorithm; nullopt for anything else.
    std::optional<HashAlgorithm> parse_hash_algorithm(std::string_view name);

    const char *to_string(HashAlgorithm algorithm);

    // Length of the algorithm's digest in hex characters (40 / 64).
    std::size_t hex_digest_length(HashAlgorithm algorithm);

    void to_json(nlohmann::json &j, HashAlgorithm algorithm);
    // Throws on an unknown algorithm name — a checksum the daemon cannot verify
    // must fail the request, never silently degrade.
    void from_json(const nlohmann::json &j, HashAlgorithm &algorithm);

    struct Checksum {
        HashAlgorithm algorithm;
        std::string hex;

        static constexpr auto kFields =
            fields(field("algorithm", &Checksum::algorithm, kRequired), field("hex", &Checksum::hex));
    };

    // A checksum is well-formed when its hex is exactly the algorithm's digest
    // length and contains only hex characters — one definition every caller
    // (engine, daemon, client, CLI) validates against.
    bool is_valid_checksum(const Checksum &checksum);

    // A request to download a URL to disk, optionally verified against a checksum.
    struct DownloadSpec {
        std::string id; // caller-assigned; empty lets the daemon generate one
        std::string url;
        std::filesystem::path destination;
        std::optional<Checksum> checksum;

        static constexpr auto kFields =
            fields(field("id", &DownloadSpec::id), field("url", &DownloadSpec::url),
                   field("dest", &DownloadSpec::destination), field("checksum", &DownloadSpec::checksum));
    };

    struct DownloadProgress {
        std::uint64_t downloaded = 0;
        std::uint64_t total = 0; // 0 = unknown

        static constexpr auto kFields =
            fields(field("downloaded", &DownloadProgress::downloaded), field("total", &DownloadProgress::total));
    };

    struct DownloadStart {
        static constexpr const char *kChannel = "download.start";
        using Params = DownloadSpec;
        struct Result {
            std::string id;

            static constexpr auto kFields = fields(field("id", &Result::id));
        };
    };

    struct DownloadProgressEvent {
        static constexpr const char *kTopic = "download.progress";
        std::string id;
        DownloadProgress progress;

        static constexpr auto kFields = fields(field("id", &DownloadProgressEvent::id),
                                               field("", &DownloadProgressEvent::progress, kFlatten));
    };

    struct DownloadDoneEvent {
        static constexpr const char *kTopic = "download.done";
        std::string id;
        std::filesystem::path path;

        static constexpr auto kFields =
            fields(field("id", &DownloadDoneEvent::id), field("path", &DownloadDoneEvent::path));
    };

    struct DownloadErrorEvent {
        static constexpr const char *kTopic = "download.error";
        std::string id;
        std::string message;

        static constexpr auto kFields =
            fields(field("id", &DownloadErrorEvent::id), field("message", &DownloadErrorEvent::message));
    };
} // namespace hestia::proto
