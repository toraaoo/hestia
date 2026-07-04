#pragma once

#include <functional>
#include <string>

#include <hestia/proto/accounts.h>

// The Microsoft -> Xbox Live -> XSTS -> Minecraft services chain (the flow
// Prism Launcher ships): a device-code OAuth grant against the consumers
// tenant, exchanged step by step for a Minecraft access token and profile.
namespace hestia::engine {
    struct DeviceAuthorization {
        proto::AccountLoginCode code;
        std::string device_code;
        int interval = 5; // seconds between token polls
    };

    struct MsaTokens {
        std::string access_token;
        std::string refresh_token;
    };

    struct MinecraftToken {
        std::string access_token;
        long long expires_in = 0; // seconds
    };

    struct MinecraftProfile {
        std::string uuid;
        std::string name;
    };

    DeviceAuthorization msa_request_device_code(const std::string &client_id);

    // Polls the token endpoint until the user approves, the code expires, or
    // `cancelled` returns true (which throws).
    MsaTokens msa_poll_for_tokens(const std::string &client_id, const DeviceAuthorization &authorization,
                                  const std::function<bool()> &cancelled);

    // Xbox Live user token -> XSTS token -> login_with_xbox.
    MinecraftToken minecraft_login(const std::string &msa_access_token);

    MinecraftProfile minecraft_profile(const std::string &minecraft_access_token);
} // namespace hestia::engine
