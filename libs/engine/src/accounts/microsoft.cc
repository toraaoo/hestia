#include "accounts/microsoft.h"

#include <chrono>
#include <stdexcept>
#include <thread>

#include <cpr/cpr.h>
#include <fmt/format.h>
#include <nlohmann/json.hpp>

namespace hestia::engine {
    using nlohmann::json;

    namespace {
        constexpr const char *kDeviceCodeUrl = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
        constexpr const char *kTokenUrl = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
        constexpr const char *kScope = "XboxLive.signin offline_access";
        constexpr const char *kXboxUserAuthUrl = "https://user.auth.xboxlive.com/user/authenticate";
        constexpr const char *kXstsAuthUrl = "https://xsts.auth.xboxlive.com/xsts/authorize";
        constexpr const char *kMinecraftLoginUrl = "https://api.minecraftservices.com/authentication/login_with_xbox";
        constexpr const char *kProfileUrl = "https://api.minecraftservices.com/minecraft/profile";

        json parse_body(const cpr::Response &response, const char *what) {
            if (response.error) {
                throw std::runtime_error(fmt::format("{} failed: {}", what, response.error.message));
            }
            const auto body = json::parse(response.text, nullptr, false);
            if (body.is_discarded()) {
                throw std::runtime_error(fmt::format("{} returned malformed JSON (HTTP {})", what,
                                                     response.status_code));
            }
            return body;
        }

        json post_form(const char *url, cpr::Payload payload, const char *what) {
            return parse_body(cpr::Post(cpr::Url{url}, std::move(payload),
                                        cpr::Header{{"Accept", "application/json"}}),
                              what);
        }

        std::pair<long, json> post_json(const char *url, const json &body, const char *what) {
            const auto response = cpr::Post(cpr::Url{url}, cpr::Body{body.dump()},
                                            cpr::Header{{"Content-Type", "application/json"},
                                                        {"Accept", "application/json"}});
            return {response.status_code, parse_body(response, what)};
        }

        std::string require_string(const json &body, const char *key, const char *what) {
            const auto it = body.find(key);
            if (it == body.end() || !it->is_string() || it->get_ref<const std::string &>().empty()) {
                throw std::runtime_error(fmt::format("{} response is missing {}", what, key));
            }
            return it->get<std::string>();
        }

        // The XErr values Xbox uses to refuse an XSTS token, mapped to messages
        // a user can act on (the same set every launcher special-cases).
        std::string xsts_error_message(long long xerr) {
            switch (xerr) {
                case 2148916233:
                    return "this Microsoft account has no Xbox profile; sign in once at https://www.xbox.com and retry";
                case 2148916235:
                    return "Xbox Live is not available in this account's country or region";
                case 2148916236:
                case 2148916237:
                    return "this account needs adult verification on the Xbox homepage";
                case 2148916238:
                    return "this is a child account; an adult must add it to a Microsoft family first";
                default:
                    return fmt::format("Xbox denied the sign-in (XErr {})", xerr);
            }
        }

        void sleep_unless_cancelled(std::chrono::seconds duration, const std::function<bool()> &cancelled) {
            const auto deadline = std::chrono::steady_clock::now() + duration;
            while (std::chrono::steady_clock::now() < deadline) {
                if (cancelled && cancelled()) {
                    throw std::runtime_error("the sign-in was cancelled");
                }
                std::this_thread::sleep_for(std::chrono::milliseconds(250));
            }
        }
    } // namespace

    DeviceAuthorization msa_request_device_code(const std::string &client_id) {
        const auto body = post_form(kDeviceCodeUrl, {{"client_id", client_id}, {"scope", kScope}},
                                    "Microsoft device-code request");
        if (body.contains("error")) {
            throw std::runtime_error(fmt::format(
                "Microsoft rejected the device-code request: {}",
                body.value("error_description", body.value("error", std::string{"unknown error"}))));
        }
        DeviceAuthorization authorization;
        authorization.device_code = require_string(body, "device_code", "Microsoft device-code");
        authorization.code.user_code = require_string(body, "user_code", "Microsoft device-code");
        authorization.code.verification_uri = require_string(body, "verification_uri", "Microsoft device-code");
        authorization.code.expires_in = body.value("expires_in", 900);
        authorization.interval = body.value("interval", 5);
        return authorization;
    }

    MsaTokens msa_poll_for_tokens(const std::string &client_id, const DeviceAuthorization &authorization,
                                  const std::function<bool()> &cancelled) {
        auto interval = std::chrono::seconds(authorization.interval > 0 ? authorization.interval : 5);
        const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(authorization.code.expires_in);
        while (std::chrono::steady_clock::now() < deadline) {
            sleep_unless_cancelled(interval, cancelled);
            json body;
            try {
                body = post_form(kTokenUrl,
                                 {{"client_id", client_id},
                                  {"grant_type", "urn:ietf:params:oauth:grant-type:device_code"},
                                  {"device_code", authorization.device_code}},
                                 "Microsoft token poll");
            } catch (const std::exception &) {
                continue; // transient network failure; the deadline still bounds us
            }
            if (!body.contains("error")) {
                return MsaTokens{.access_token = require_string(body, "access_token", "Microsoft token"),
                                 .refresh_token = body.value("refresh_token", std::string{})};
            }
            const auto error = body["error"].get<std::string>();
            if (error == "authorization_pending") continue;
            if (error == "slow_down") {
                interval += std::chrono::seconds(5);
                continue;
            }
            if (error == "authorization_declined") {
                throw std::runtime_error("the sign-in was declined");
            }
            if (error == "expired_token") break;
            throw std::runtime_error(fmt::format("Microsoft sign-in failed: {}",
                                                 body.value("error_description", error)));
        }
        throw std::runtime_error("the sign-in code expired before the sign-in completed; try again");
    }

    MinecraftToken minecraft_login(const std::string &msa_access_token) {
        const auto [xbl_status, xbl] = post_json(kXboxUserAuthUrl,
                                                 {{"Properties",
                                                   {{"AuthMethod", "RPS"},
                                                    {"SiteName", "user.auth.xboxlive.com"},
                                                    {"RpsTicket", "d=" + msa_access_token}}},
                                                  {"RelyingParty", "http://auth.xboxlive.com"},
                                                  {"TokenType", "JWT"}},
                                                 "Xbox Live sign-in");
        if (xbl_status != 200) {
            throw std::runtime_error(fmt::format("Xbox Live sign-in failed (HTTP {})", xbl_status));
        }
        const auto xbl_token = require_string(xbl, "Token", "Xbox Live");

        const auto [xsts_status, xsts] = post_json(kXstsAuthUrl,
                                                   {{"Properties",
                                                     {{"SandboxId", "RETAIL"},
                                                      {"UserTokens", json::array({xbl_token})}}},
                                                    {"RelyingParty", "rp://api.minecraftservices.com/"},
                                                    {"TokenType", "JWT"}},
                                                   "Xbox XSTS authorization");
        if (xsts_status == 401) {
            throw std::runtime_error(xsts_error_message(xsts.value("XErr", 0LL)));
        }
        if (xsts_status != 200) {
            throw std::runtime_error(fmt::format("Xbox XSTS authorization failed (HTTP {})", xsts_status));
        }
        const auto xsts_token = require_string(xsts, "Token", "Xbox XSTS");
        std::string user_hash;
        try {
            user_hash = xsts.at("DisplayClaims").at("xui").at(0).at("uhs").get<std::string>();
        } catch (const json::exception &) {
            throw std::runtime_error("Xbox XSTS response is missing the user hash");
        }

        const auto [mc_status, mc] = post_json(kMinecraftLoginUrl,
                                               {{"identityToken",
                                                 fmt::format("XBL3.0 x={};{}", user_hash, xsts_token)}},
                                               "Minecraft services sign-in");
        if (mc_status != 200) {
            throw std::runtime_error(fmt::format("Minecraft services sign-in failed (HTTP {})", mc_status));
        }
        return MinecraftToken{.access_token = require_string(mc, "access_token", "Minecraft services"),
                              .expires_in = mc.value("expires_in", 0LL)};
    }

    MinecraftProfile minecraft_profile(const std::string &minecraft_access_token) {
        const auto response = cpr::Get(cpr::Url{kProfileUrl},
                                       cpr::Header{{"Authorization", "Bearer " + minecraft_access_token},
                                                   {"Accept", "application/json"}});
        if (response.status_code == 404) {
            throw std::runtime_error("this Microsoft account owns no Minecraft profile; buy Minecraft: Java "
                                     "Edition or create the profile in the official launcher first");
        }
        const auto body = parse_body(response, "Minecraft profile fetch");
        if (response.status_code != 200) {
            throw std::runtime_error(fmt::format("Minecraft profile fetch failed (HTTP {})", response.status_code));
        }
        return MinecraftProfile{.uuid = require_string(body, "id", "Minecraft profile"),
                                .name = require_string(body, "name", "Minecraft profile")};
    }
} // namespace hestia::engine
