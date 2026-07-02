#include <hestia/engine/checksum.h>

#include <algorithm>
#include <cstring>

namespace hestia::engine {
    namespace {
        constexpr std::uint32_t kSha256K[64] = {
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1,
            0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
            0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
            0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
            0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147,
            0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
            0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
            0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
            0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
            0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
            0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
        };

        std::uint32_t rotl(std::uint32_t x, int n) { return (x << n) | (x >> (32 - n)); }
        std::uint32_t rotr(std::uint32_t x, int n) { return (x >> n) | (x << (32 - n)); }

        std::uint32_t load_be32(const std::uint8_t *p) {
            return (std::uint32_t{p[0]} << 24) | (std::uint32_t{p[1]} << 16) |
                   (std::uint32_t{p[2]} << 8) | std::uint32_t{p[3]};
        }
    }

    std::optional<HashAlgorithm> parse_hash_algorithm(std::string_view name) {
        if (name == "sha1") return HashAlgorithm::sha1;
        if (name == "sha256") return HashAlgorithm::sha256;
        return std::nullopt;
    }

    std::size_t hex_digest_length(HashAlgorithm algorithm) {
        return algorithm == HashAlgorithm::sha1 ? 40 : 64;
    }

    Hasher::Hasher(HashAlgorithm algorithm) : algorithm_(algorithm) {
        if (algorithm_ == HashAlgorithm::sha1) {
            const std::uint32_t init[8] = {0x67452301, 0xefcdab89, 0x98badcfe,
                                           0x10325476, 0xc3d2e1f0, 0, 0, 0};
            std::memcpy(state_, init, sizeof(state_));
        } else {
            const std::uint32_t init[8] = {0x6a09e667, 0xbb67ae85, 0x3c6ef372,
                                           0xa54ff53a, 0x510e527f, 0x9b05688c,
                                           0x1f83d9ab, 0x5be0cd19};
            std::memcpy(state_, init, sizeof(state_));
        }
    }

    void Hasher::update(const void *data, std::size_t len) {
        const auto *p = static_cast<const std::uint8_t *>(data);
        total_bytes_ += len;
        if (buffer_len_ > 0) {
            const std::size_t take = std::min(len, sizeof(buffer_) - buffer_len_);
            std::memcpy(buffer_ + buffer_len_, p, take);
            buffer_len_ += take;
            p += take;
            len -= take;
            if (buffer_len_ == sizeof(buffer_)) {
                process_block(buffer_);
                buffer_len_ = 0;
            }
        }
        while (len >= sizeof(buffer_)) {
            process_block(p);
            p += sizeof(buffer_);
            len -= sizeof(buffer_);
        }
        if (len > 0) {
            std::memcpy(buffer_, p, len);
            buffer_len_ = len;
        }
    }

    std::string Hasher::hex_digest() {
        const std::uint64_t bit_length = total_bytes_ * 8;
        const std::uint8_t one = 0x80;
        const std::uint8_t zero = 0;
        update(&one, 1);
        while (buffer_len_ != 56) update(&zero, 1);
        std::uint8_t length_be[8];
        for (int i = 0; i < 8; ++i) {
            length_be[i] = static_cast<std::uint8_t>(bit_length >> (56 - 8 * i));
        }
        update(length_be, sizeof(length_be));

        const std::size_t words = algorithm_ == HashAlgorithm::sha1 ? 5 : 8;
        static constexpr char kHex[] = "0123456789abcdef";
        std::string out;
        out.reserve(words * 8);
        for (std::size_t w = 0; w < words; ++w) {
            for (int shift = 28; shift >= 0; shift -= 4) {
                out.push_back(kHex[(state_[w] >> shift) & 0xf]);
            }
        }
        return out;
    }

    void Hasher::process_block(const std::uint8_t *block) {
        if (algorithm_ == HashAlgorithm::sha1) {
            std::uint32_t w[80];
            for (std::size_t i = 0; i < 16; ++i) w[i] = load_be32(block + 4u * i);
            for (int i = 16; i < 80; ++i) {
                w[i] = rotl(w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16], 1);
            }
            std::uint32_t a = state_[0], b = state_[1], c = state_[2], d = state_[3],
                          e = state_[4];
            for (int i = 0; i < 80; ++i) {
                std::uint32_t f, k;
                if (i < 20) {
                    f = (b & c) | (~b & d);
                    k = 0x5a827999;
                } else if (i < 40) {
                    f = b ^ c ^ d;
                    k = 0x6ed9eba1;
                } else if (i < 60) {
                    f = (b & c) | (b & d) | (c & d);
                    k = 0x8f1bbcdc;
                } else {
                    f = b ^ c ^ d;
                    k = 0xca62c1d6;
                }
                const std::uint32_t temp = rotl(a, 5) + f + e + k + w[i];
                e = d;
                d = c;
                c = rotl(b, 30);
                b = a;
                a = temp;
            }
            state_[0] += a;
            state_[1] += b;
            state_[2] += c;
            state_[3] += d;
            state_[4] += e;
            return;
        }

        std::uint32_t w[64];
        for (std::size_t i = 0; i < 16; ++i) w[i] = load_be32(block + 4u * i);
        for (int i = 16; i < 64; ++i) {
            const std::uint32_t s0 =
                rotr(w[i - 15], 7) ^ rotr(w[i - 15], 18) ^ (w[i - 15] >> 3);
            const std::uint32_t s1 =
                rotr(w[i - 2], 17) ^ rotr(w[i - 2], 19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16] + s0 + w[i - 7] + s1;
        }
        std::uint32_t a = state_[0], b = state_[1], c = state_[2], d = state_[3],
                      e = state_[4], f = state_[5], g = state_[6], h = state_[7];
        for (int i = 0; i < 64; ++i) {
            const std::uint32_t s1 = rotr(e, 6) ^ rotr(e, 11) ^ rotr(e, 25);
            const std::uint32_t ch = (e & f) ^ (~e & g);
            const std::uint32_t temp1 = h + s1 + ch + kSha256K[i] + w[i];
            const std::uint32_t s0 = rotr(a, 2) ^ rotr(a, 13) ^ rotr(a, 22);
            const std::uint32_t maj = (a & b) ^ (a & c) ^ (b & c);
            const std::uint32_t temp2 = s0 + maj;
            h = g;
            g = f;
            f = e;
            e = d + temp1;
            d = c;
            c = b;
            b = a;
            a = temp1 + temp2;
        }
        state_[0] += a;
        state_[1] += b;
        state_[2] += c;
        state_[3] += d;
        state_[4] += e;
        state_[5] += f;
        state_[6] += g;
        state_[7] += h;
    }
}
