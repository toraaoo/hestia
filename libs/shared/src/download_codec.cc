#include "hestia/ipc/download_codec.h"

#include <cctype>
#include <stdexcept>

namespace hestia::ipc {
    using nlohmann::json;

    namespace {
        bool is_hex(std::string_view s) {
            for (const char c : s) {
                if (!std::isxdigit(static_cast<unsigned char>(c))) return false;
            }
            return true;
        }
    }

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

    bool is_valid_checksum(const Checksum &checksum) {
        return checksum.hex.size() == hex_digest_length(checksum.algorithm) &&
               is_hex(checksum.hex);
    }

    json to_json(const DownloadSpec &s) {
        json j{
            {"id", s.id},
            {"url", s.url},
            {"dest", s.destination.string()},
        };
        if (s.checksum) {
            j["checksum"] = {
                {"algorithm", to_string(s.checksum->algorithm)},
                {"hex", s.checksum->hex},
            };
        }
        return j;
    }

    DownloadSpec download_spec_from_json(const json &payload) {
        DownloadSpec spec;
        spec.id = payload.value("id", std::string{});
        spec.url = payload.value("url", std::string{});
        spec.destination = payload.value("dest", std::string{});
        if (payload.contains("checksum")) {
            const auto &c = payload.at("checksum");
            const auto name = c.value("algorithm", std::string{});
            const auto algorithm = parse_hash_algorithm(name);
            if (!algorithm) {
                throw std::runtime_error("unknown checksum algorithm: " + name);
            }
            spec.checksum = Checksum{*algorithm, c.value("hex", std::string{})};
        }
        return spec;
    }

    json to_json(const DownloadProgress &p) {
        return json{
            {"downloaded", p.downloaded},
            {"total", p.total},
        };
    }

    DownloadProgress progress_from_json(const json &j) {
        DownloadProgress p;
        p.downloaded = j.value("downloaded", std::uint64_t{0});
        p.total = j.value("total", std::uint64_t{0});
        return p;
    }
}
