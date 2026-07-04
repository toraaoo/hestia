#pragma once

#include <cstddef>
#include <cstdint>
#include <string>

#include <hestia/proto/download.h>

namespace hestia::engine {
    // Incremental SHA-1/SHA-256; hex_digest() finalizes — no update() after it.
    class Hasher {
    public:
        explicit Hasher(proto::HashAlgorithm algorithm);

        void update(const void *data, std::size_t len);
        std::string hex_digest();

    private:
        void process_block(const std::uint8_t *block);

        proto::HashAlgorithm algorithm_;
        std::uint32_t state_[8];
        std::uint64_t total_bytes_ = 0;
        std::uint8_t buffer_[64];
        std::size_t buffer_len_ = 0;
    };
} // namespace hestia::engine
