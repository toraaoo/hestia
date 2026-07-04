#include "accounts/signing.h"

#include <stdexcept>

#include <windows.h>

#include <bcrypt.h>

namespace hestia::engine {
    namespace {
        bool nt_ok(NTSTATUS status) { return status >= 0; }

        std::vector<std::uint8_t> sha256(const std::vector<std::uint8_t> &data) {
            std::vector<std::uint8_t> digest(32);
            if (!nt_ok(BCryptHash(BCRYPT_SHA256_ALG_HANDLE, nullptr, 0,
                                  const_cast<PUCHAR>(data.data()), static_cast<ULONG>(data.size()),
                                  digest.data(), static_cast<ULONG>(digest.size())))) {
                throw std::runtime_error("SHA-256 hashing failed");
            }
            return digest;
        }
    } // namespace

    struct ProofKey::Impl {
        BCRYPT_KEY_HANDLE key = nullptr;
        ~Impl() {
            if (key != nullptr) BCryptDestroyKey(key);
        }
    };

    ProofKey::ProofKey() : impl_(std::make_unique<Impl>()) {}
    ProofKey::ProofKey(ProofKey &&) noexcept = default;
    ProofKey &ProofKey::operator=(ProofKey &&) noexcept = default;
    ProofKey::~ProofKey() = default;

    ProofKey ProofKey::generate() {
        ProofKey key;
        if (!nt_ok(BCryptGenerateKeyPair(BCRYPT_ECDSA_P256_ALG_HANDLE, &key.impl_->key, 256, 0)) ||
            !nt_ok(BCryptFinalizeKeyPair(key.impl_->key, 0))) {
            throw std::runtime_error("failed to generate an ECDSA P-256 key");
        }

        ULONG blob_len = 0;
        if (!nt_ok(BCryptExportKey(key.impl_->key, nullptr, BCRYPT_ECCPUBLIC_BLOB, nullptr, 0, &blob_len, 0))) {
            throw std::runtime_error("failed to size the ECDSA public key");
        }
        std::vector<std::uint8_t> blob(blob_len);
        if (!nt_ok(BCryptExportKey(key.impl_->key, nullptr, BCRYPT_ECCPUBLIC_BLOB, blob.data(), blob_len,
                                   &blob_len, 0))) {
            throw std::runtime_error("failed to export the ECDSA public key");
        }

        const std::size_t header = sizeof(BCRYPT_ECCKEY_BLOB);
        if (blob.size() < header + 64) {
            throw std::runtime_error("unexpected ECDSA public key layout");
        }
        const std::uint8_t *point = blob.data() + header;

        key.id_ = format_uuid_v4(random_bytes(16));
        key.x_ = base64url_nopad({point, point + 32});
        key.y_ = base64url_nopad({point + 32, point + 64});
        return key;
    }

    std::vector<std::uint8_t> ProofKey::sign(const std::vector<std::uint8_t> &message) const {
        const auto digest = sha256(message);
        ULONG sig_len = 0;
        if (!nt_ok(BCryptSignHash(impl_->key, nullptr, const_cast<PUCHAR>(digest.data()),
                                  static_cast<ULONG>(digest.size()), nullptr, 0, &sig_len, 0))) {
            throw std::runtime_error("failed to size the ECDSA signature");
        }
        std::vector<std::uint8_t> raw(sig_len);
        if (!nt_ok(BCryptSignHash(impl_->key, nullptr, const_cast<PUCHAR>(digest.data()),
                                  static_cast<ULONG>(digest.size()), raw.data(), sig_len, &sig_len, 0))) {
            throw std::runtime_error("ECDSA signing failed");
        }
        raw.resize(sig_len);
        return raw;
    }

    std::vector<std::uint8_t> random_bytes(std::size_t count) {
        std::vector<std::uint8_t> out(count);
        if (!nt_ok(BCryptGenRandom(BCRYPT_RNG_ALG_HANDLE, out.data(), static_cast<ULONG>(count), 0))) {
            throw std::runtime_error("failed to gather secure random bytes");
        }
        return out;
    }
} // namespace hestia::engine
