#include "hestia/proto/download.h"

#include <cctype>
#include <stdexcept>

namespace hestia::proto {
    namespace {
        bool is_hex(std::string_view s) {
            for (const char c: s) {
                if (!std::isxdigit(static_cast<unsigned char>(c))) return false;
            }
            return true;
        }
    } // namespace

    std::optional<HashAlgorithm> parse_hash_algorithm(std::string_view name) {
        if (name == "sha1") return HashAlgorithm::sha1;
        if (name == "sha256") return HashAlgorithm::sha256;
        return std::nullopt;
    }

    const char *to_string(HashAlgorithm algorithm) {
        return algorithm == HashAlgorithm::sha1 ? "sha1" : "sha256";
    }

    std::size_t hex_digest_length(HashAlgorithm algorithm) {
        return algorithm == HashAlgorithm::sha1 ? 40 : 64;
    }

    void to_json(nlohmann::json &j, HashAlgorithm algorithm) {
        j = to_string(algorithm);
    }

    void from_json(const nlohmann::json &j, HashAlgorithm &algorithm) {
        const auto name = j.get<std::string>();
        const auto parsed = parse_hash_algorithm(name);
        if (!parsed) {
            throw std::runtime_error("unknown checksum algorithm: " + name);
        }
        algorithm = *parsed;
    }

    bool is_valid_checksum(const Checksum &checksum) {
        return checksum.hex.size() == hex_digest_length(checksum.algorithm) && is_hex(checksum.hex);
    }
} // namespace hestia::proto
