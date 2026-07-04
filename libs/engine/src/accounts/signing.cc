#include "accounts/signing.h"

#include <stdexcept>

namespace hestia::engine {
    namespace {
        constexpr char kStandardAlphabet[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        constexpr char kUrlAlphabet[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

        std::string base64_encode(const std::vector<std::uint8_t> &data, const char *alphabet, bool pad) {
            std::string out;
            out.reserve((data.size() + 2) / 3 * 4);
            std::size_t i = 0;
            for (; i + 3 <= data.size(); i += 3) {
                const std::uint32_t n = (data[i] << 16) | (data[i + 1] << 8) | data[i + 2];
                out.push_back(alphabet[(n >> 18) & 0x3F]);
                out.push_back(alphabet[(n >> 12) & 0x3F]);
                out.push_back(alphabet[(n >> 6) & 0x3F]);
                out.push_back(alphabet[n & 0x3F]);
            }
            if (const std::size_t rest = data.size() - i; rest == 1) {
                const std::uint32_t n = data[i] << 16;
                out.push_back(alphabet[(n >> 18) & 0x3F]);
                out.push_back(alphabet[(n >> 12) & 0x3F]);
                if (pad) out.append("==");
            } else if (rest == 2) {
                const std::uint32_t n = (data[i] << 16) | (data[i + 1] << 8);
                out.push_back(alphabet[(n >> 18) & 0x3F]);
                out.push_back(alphabet[(n >> 12) & 0x3F]);
                out.push_back(alphabet[(n >> 6) & 0x3F]);
                if (pad) out.push_back('=');
            }
            return out;
        }

        void append_be32(std::vector<std::uint8_t> &buf, std::uint32_t value) {
            buf.push_back(static_cast<std::uint8_t>(value >> 24));
            buf.push_back(static_cast<std::uint8_t>(value >> 16));
            buf.push_back(static_cast<std::uint8_t>(value >> 8));
            buf.push_back(static_cast<std::uint8_t>(value));
        }

        void append_be64(std::vector<std::uint8_t> &buf, std::uint64_t value) {
            for (int shift = 56; shift >= 0; shift -= 8) {
                buf.push_back(static_cast<std::uint8_t>(value >> shift));
            }
        }

        void append_bytes(std::vector<std::uint8_t> &buf, const std::string &value) {
            buf.insert(buf.end(), value.begin(), value.end());
        }
    } // namespace

    std::string format_uuid_v4(std::vector<std::uint8_t> bytes) {
        if (bytes.size() != 16) {
            throw std::runtime_error("a UUID needs exactly 16 bytes");
        }
        bytes[6] = static_cast<std::uint8_t>((bytes[6] & 0x0F) | 0x40);
        bytes[8] = static_cast<std::uint8_t>((bytes[8] & 0x3F) | 0x80);
        constexpr char hex[] = "0123456789abcdef";
        std::string out;
        out.reserve(36);
        for (std::size_t i = 0; i < bytes.size(); ++i) {
            if (i == 4 || i == 6 || i == 8 || i == 10) out.push_back('-');
            out.push_back(hex[bytes[i] >> 4]);
            out.push_back(hex[bytes[i] & 0x0F]);
        }
        return out;
    }

    std::string base64_standard(const std::vector<std::uint8_t> &data) {
        return base64_encode(data, kStandardAlphabet, true);
    }

    std::string base64url_nopad(const std::vector<std::uint8_t> &data) {
        return base64_encode(data, kUrlAlphabet, false);
    }

    std::string xbox_signature_header(const ProofKey &key, const std::string &url_path,
                                      const std::string &authorization, const std::string &body,
                                      std::int64_t unix_time) {
        const auto filetime = static_cast<std::uint64_t>(unix_time + 11644473600LL) * 10000000ULL;

        std::vector<std::uint8_t> message;
        append_be32(message, 1);
        message.push_back(0);
        append_be64(message, filetime);
        message.push_back(0);
        append_bytes(message, "POST");
        message.push_back(0);
        append_bytes(message, url_path);
        message.push_back(0);
        append_bytes(message, authorization);
        message.push_back(0);
        append_bytes(message, body);
        message.push_back(0);

        const auto signature = key.sign(message);
        if (signature.size() != 64) {
            throw std::runtime_error("ECDSA signature was not 64 bytes");
        }

        std::vector<std::uint8_t> envelope;
        append_be32(envelope, 1);
        append_be64(envelope, filetime);
        envelope.insert(envelope.end(), signature.begin(), signature.end());
        return base64_standard(envelope);
    }
} // namespace hestia::engine
