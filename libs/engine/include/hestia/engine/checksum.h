#pragma once

#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <string_view>

namespace hestia::engine {
    enum class HashAlgorithm : std::uint8_t { sha1, sha256 };

    // "sha1" / "sha256" → the algorithm; nullopt for anything else.
    std::optional<HashAlgorithm> parse_hash_algorithm(std::string_view name);

    // Length of the algorithm's digest in hex characters (40 / 64).
    std::size_t hex_digest_length(HashAlgorithm algorithm);

    // Incremental SHA-1/SHA-256, so a download can be hashed as it streams
    // instead of re-reading the file. hex_digest() finalizes: the hasher must
    // not be updated afterwards.
    class Hasher {
    public:
        explicit Hasher(HashAlgorithm algorithm);

        void update(const void *data, std::size_t len);
        std::string hex_digest();

    private:
        void process_block(const std::uint8_t *block);

        HashAlgorithm algorithm_;
        std::uint32_t state_[8];
        std::uint64_t total_bytes_ = 0;
        std::uint8_t buffer_[64];
        std::size_t buffer_len_ = 0;
    };
}
