#include "accounts/signing.h"

#include <stdexcept>

#include <openssl/core_names.h>
#include <openssl/ec.h>
#include <openssl/ecdsa.h>
#include <openssl/evp.h>
#include <openssl/rand.h>

namespace hestia::engine {
    struct ProofKey::Impl {
        EVP_PKEY *pkey = nullptr;
        ~Impl() { EVP_PKEY_free(pkey); }
    };

    ProofKey::ProofKey() : impl_(std::make_unique<Impl>()) {}
    ProofKey::ProofKey(ProofKey &&) noexcept = default;
    ProofKey &ProofKey::operator=(ProofKey &&) noexcept = default;
    ProofKey::~ProofKey() = default;

    ProofKey ProofKey::generate() {
        ProofKey key;
        key.impl_->pkey = EVP_EC_gen("P-256");
        if (key.impl_->pkey == nullptr) {
            throw std::runtime_error("failed to generate an ECDSA P-256 key");
        }

        std::uint8_t point[65];
        std::size_t point_len = 0;
        if (EVP_PKEY_get_octet_string_param(key.impl_->pkey, OSSL_PKEY_PARAM_PUB_KEY, point, sizeof(point),
                                            &point_len) != 1 ||
            point_len != sizeof(point) || point[0] != 0x04) {
            throw std::runtime_error("failed to read the ECDSA public point");
        }

        key.id_ = format_uuid_v4(random_bytes(16));
        key.x_ = base64url_nopad({point + 1, point + 33});
        key.y_ = base64url_nopad({point + 33, point + 65});
        return key;
    }

    std::vector<std::uint8_t> ProofKey::sign(const std::vector<std::uint8_t> &message) const {
        EVP_MD_CTX *ctx = EVP_MD_CTX_new();
        if (ctx == nullptr) {
            throw std::runtime_error("failed to allocate a signing context");
        }

        std::vector<std::uint8_t> der;
        std::size_t der_len = 0;
        const bool ok =
            EVP_DigestSignInit(ctx, nullptr, EVP_sha256(), nullptr, impl_->pkey) == 1 &&
            EVP_DigestSign(ctx, nullptr, &der_len, message.data(), message.size()) == 1 &&
            (der.resize(der_len), EVP_DigestSign(ctx, der.data(), &der_len, message.data(), message.size()) == 1);
        EVP_MD_CTX_free(ctx);
        if (!ok) {
            throw std::runtime_error("ECDSA signing failed");
        }
        der.resize(der_len);

        const std::uint8_t *der_ptr = der.data();
        ECDSA_SIG *sig = d2i_ECDSA_SIG(nullptr, &der_ptr, static_cast<long>(der.size()));
        if (sig == nullptr) {
            throw std::runtime_error("failed to parse the ECDSA signature");
        }
        const BIGNUM *r = nullptr;
        const BIGNUM *s = nullptr;
        ECDSA_SIG_get0(sig, &r, &s);

        std::vector<std::uint8_t> raw(64);
        if (BN_bn2binpad(r, raw.data(), 32) != 32 || BN_bn2binpad(s, raw.data() + 32, 32) != 32) {
            ECDSA_SIG_free(sig);
            throw std::runtime_error("failed to serialize the ECDSA signature");
        }
        ECDSA_SIG_free(sig);
        return raw;
    }

    std::vector<std::uint8_t> random_bytes(std::size_t count) {
        std::vector<std::uint8_t> out(count);
        if (RAND_bytes(out.data(), static_cast<int>(count)) != 1) {
            throw std::runtime_error("failed to gather secure random bytes");
        }
        return out;
    }
} // namespace hestia::engine
