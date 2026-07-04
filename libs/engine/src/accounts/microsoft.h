#pragma once

#include <cstdint>
#include <optional>
#include <string>

#include "accounts/signing.h"

namespace hestia::engine {
    struct DeviceToken {
        std::string token;
        std::int64_t clock_offset = 0;
    };

    struct SisuAuthentication {
        std::string session_id;
        std::string url;
    };

    struct DeviceCodeChallenge {
        std::string user_code;
        std::string verification_uri;
        std::string device_code;
        long long interval_seconds = 5;
        long long expires_in_seconds = 900;
    };

    struct OAuthTokens {
        std::string access_token;
        std::string refresh_token;
        long long expires_in = 0;
    };

    struct SisuAuthorization {
        std::string user_token;
        std::string title_token;
    };

    struct XstsToken {
        std::string token;
        std::string user_hash;
    };

    struct MinecraftProfile {
        std::string uuid;
        std::string name;
    };

    DeviceToken request_device_token(const ProofKey &key);

    SisuAuthentication sisu_authenticate(const std::string &device_token, const std::string &challenge,
                                         const std::string &state, const ProofKey &key,
                                         std::int64_t clock_offset);

    DeviceCodeChallenge request_device_code();

    std::optional<OAuthTokens> poll_device_code(const std::string &device_code);

    OAuthTokens redeem_code(const std::string &code, const std::string &verifier);

    OAuthTokens refresh_oauth(const std::string &refresh_token);

    SisuAuthorization sisu_authorize(const std::string &session_id, const std::string &access_token,
                                     const std::string &device_token, const ProofKey &key,
                                     std::int64_t clock_offset);

    XstsToken xsts_authorize(const SisuAuthorization &authorization, const std::string &device_token,
                             const ProofKey &key, std::int64_t clock_offset);

    std::string launcher_login(const XstsToken &xsts);

    MinecraftProfile minecraft_profile(const std::string &minecraft_access_token);
} // namespace hestia::engine
