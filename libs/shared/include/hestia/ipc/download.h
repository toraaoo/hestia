#pragma once

#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <optional>
#include <string>
#include <string_view>

// The download domain types, shared by the daemon (which runs downloads) and the
// client SDK (which requests them). Wire codec: download_codec.h.
namespace hestia::ipc {
    enum class HashAlgorithm : std::uint8_t { sha1, sha256 };

    // "sha1" / "sha256" → the algorithm; nullopt for anything else.
    std::optional<HashAlgorithm> parse_hash_algorithm(std::string_view name);

    const char *to_string(HashAlgorithm algorithm);

    // Length of the algorithm's digest in hex characters (40 / 64).
    std::size_t hex_digest_length(HashAlgorithm algorithm);

    struct Checksum {
        HashAlgorithm algorithm;
        std::string hex;
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
    };

    struct DownloadProgress {
        std::uint64_t downloaded = 0;
        std::uint64_t total = 0; // 0 = unknown
    };
}
