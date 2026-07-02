#pragma once

#include <cstddef>
#include <cstdint>
#include <string>

#include <hestia/ipc/download.h>

namespace hestia::engine {
    // Incremental SHA-1/SHA-256, so a download can be hashed as it streams
    // instead of re-reading the file. hex_digest() finalizes: the hasher must
    // not be updated afterwards. The algorithm vocabulary (HashAlgorithm,
    // hex_digest_length, …) lives in <hestia/ipc/download.h>, shared with the
    // wire codec.
    class Hasher {
    public:
        explicit Hasher(ipc::HashAlgorithm algorithm);

        void update(const void *data, std::size_t len);
        std::string hex_digest();

    private:
        void process_block(const std::uint8_t *block);

        ipc::HashAlgorithm algorithm_;
        std::uint32_t state_[8];
        std::uint64_t total_bytes_ = 0;
        std::uint8_t buffer_[64];
        std::size_t buffer_len_ = 0;
    };
} // namespace hestia::engine
