#pragma once

#include <cstddef>
#include <cstdint>
#include <memory>
#include <string>
#include <vector>

namespace hestia::engine {
    class ProofKey {
    public:
        static ProofKey generate();

        ProofKey(ProofKey &&) noexcept;
        ProofKey &operator=(ProofKey &&) noexcept;
        ProofKey(const ProofKey &) = delete;
        ProofKey &operator=(const ProofKey &) = delete;
        ~ProofKey();

        [[nodiscard]] const std::string &id() const { return id_; }
        [[nodiscard]] const std::string &x() const { return x_; }
        [[nodiscard]] const std::string &y() const { return y_; }

        [[nodiscard]] std::vector<std::uint8_t> sign(const std::vector<std::uint8_t> &message) const;

    private:
        ProofKey();

        struct Impl;
        std::unique_ptr<Impl> impl_;
        std::string id_;
        std::string x_;
        std::string y_;
    };

    std::vector<std::uint8_t> random_bytes(std::size_t count);
    std::string format_uuid_v4(std::vector<std::uint8_t> bytes);

    std::string base64_standard(const std::vector<std::uint8_t> &data);
    std::string base64url_nopad(const std::vector<std::uint8_t> &data);

    std::string xbox_signature_header(const ProofKey &key, const std::string &url_path,
                                      const std::string &authorization, const std::string &body,
                                      std::int64_t unix_time);
} // namespace hestia::engine
