#include "accounts/microsoft.h"

#include <cctype>
#include <chrono>
#include <stdexcept>

#include <cpr/cpr.h>
#include <fmt/format.h>
#include <nlohmann/json.hpp>

namespace hestia::engine {
    using nlohmann::json;

    namespace {
        constexpr const char *kClientId = "00000000402b5328";
        constexpr const char *kReplyUrl = "https://login.live.com/oauth20_desktop.srf";
        constexpr const char *kScope = "service::user.auth.xboxlive.com::MBI_SSL";
        constexpr const char *kTitleId = "1794566092";
        constexpr const char *kUserAgent = "Hestia/1.0 (+https://github.com/toraaoo/hestia)";

        constexpr const char *kDeviceAuthUrl = "https://device.auth.xboxlive.com/device/authenticate";
        constexpr const char *kSisuAuthenticateUrl = "https://sisu.xboxlive.com/authenticate";
        constexpr const char *kOauthTokenUrl = "https://login.live.com/oauth20_token.srf";
        constexpr const char *kSisuAuthorizeUrl = "https://sisu.xboxlive.com/authorize";
        constexpr const char *kXstsUrl = "https://xsts.auth.xboxlive.com/xsts/authorize";
        constexpr const char *kLauncherLoginUrl = "https://api.minecraftservices.com/launcher/login";
        constexpr const char *kProfileUrl = "https://api.minecraftservices.com/minecraft/profile";

        json parse_body(const cpr::Response &response, const char *what) {
            if (response.error) {
                throw std::runtime_error(fmt::format("{} failed: {}", what, response.error.message));
            }
            const auto body = json::parse(response.text, nullptr, false);
            if (body.is_discarded()) {
                throw std::runtime_error(
                    fmt::format("{} returned malformed JSON (HTTP {})", what, response.status_code));
            }
            return body;
        }

        std::string require_string(const json &body, const char *key, const char *what) {
            const auto it = body.find(key);
            if (it == body.end() || !it->is_string() || it->get_ref<const std::string &>().empty()) {
                throw std::runtime_error(fmt::format("{} response is missing {}", what, key));
            }
            return it->get<std::string>();
        }

        std::string nested_token(const json &body, const char *key, const char *what) {
            const auto it = body.find(key);
            if (it == body.end() || !it->is_object()) {
                throw std::runtime_error(fmt::format("{} response is missing {}", what, key));
            }
            return require_string(*it, "Token", what);
        }

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

        std::string to_upper(std::string value) {
            for (auto &c: value) c = static_cast<char>(::toupper(static_cast<unsigned char>(c)));
            return value;
        }

        json proof_jwk(const ProofKey &key) {
            return json{{"kty", "EC"}, {"x", key.x()}, {"y", key.y()},
                        {"crv", "P-256"}, {"alg", "ES256"}, {"use", "sig"}};
        }

        std::int64_t now_seconds() {
            return std::chrono::duration_cast<std::chrono::seconds>(
                       std::chrono::system_clock::now().time_since_epoch())
                .count();
        }

        cpr::Response signed_post(const char *url, const char *url_path, const json &body, const ProofKey &key,
                                  bool contract_version) {
            const auto payload = body.dump();
            const auto signature = xbox_signature_header(key, url_path, "", payload, now_seconds());
            cpr::Header header{{"Content-Type", "application/json; charset=utf-8"},
                               {"Accept", "application/json"},
                               {"Signature", signature}};
            if (contract_version) header["x-xbl-contract-version"] = "1";
            return cpr::Post(cpr::Url{url}, cpr::Body{payload}, header);
        }
    } // namespace

    std::string request_device_token(const ProofKey &key) {
        const json body = {{"Properties",
                            {{"AuthMethod", "ProofOfPossession"},
                             {"Id", fmt::format("{{{}}}", to_upper(key.id()))},
                             {"DeviceType", "Win32"},
                             {"Version", "10.16.0"},
                             {"ProofKey", proof_jwk(key)}}},
                           {"RelyingParty", "http://auth.xboxlive.com"},
                           {"TokenType", "JWT"}};
        const auto response = signed_post(kDeviceAuthUrl, "/device/authenticate", body, key, true);
        return require_string(parse_body(response, "Xbox device token"), "Token", "Xbox device token");
    }

    SisuAuthentication sisu_authenticate(const std::string &device_token, const std::string &challenge,
                                         const std::string &state, const ProofKey &key) {
        const json body = {{"AppId", kClientId},
                           {"DeviceToken", device_token},
                           {"Offers", json::array({kScope})},
                           {"Query",
                            {{"code_challenge", challenge},
                             {"code_challenge_method", "S256"},
                             {"state", state},
                             {"prompt", "select_account"}}},
                           {"RedirectUri", kReplyUrl},
                           {"Sandbox", "RETAIL"},
                           {"TokenType", "code"},
                           {"TitleId", kTitleId}};
        const auto response = signed_post(kSisuAuthenticateUrl, "/authenticate", body, key, true);
        const auto doc = parse_body(response, "Xbox sign-in request");

        const auto session = response.header.find("X-SessionId");
        if (session == response.header.end() || session->second.empty()) {
            throw std::runtime_error("Xbox sign-in request did not return a session id");
        }
        return SisuAuthentication{.session_id = session->second,
                                  .url = require_string(doc, "MsaOauthRedirect", "Xbox sign-in request")};
    }

    namespace {
        OAuthTokens exchange_oauth(cpr::Payload payload, const char *what, const char *rejection) {
            const auto response =
                cpr::Post(cpr::Url{kOauthTokenUrl}, std::move(payload), cpr::Header{{"Accept", "application/json"}});
            const auto doc = parse_body(response, what);
            if (doc.contains("error")) {
                throw std::runtime_error(fmt::format(
                    "{}: {}", rejection,
                    doc.value("error_description", doc.value("error", std::string{"unknown error"}))));
            }
            return OAuthTokens{.access_token = require_string(doc, "access_token", what),
                               .refresh_token = doc.value("refresh_token", std::string{}),
                               .expires_in = doc.value("expires_in", 0LL)};
        }
    } // namespace

    OAuthTokens redeem_code(const std::string &code, const std::string &verifier) {
        return exchange_oauth(cpr::Payload{{"client_id", kClientId},
                                           {"code", code},
                                           {"code_verifier", verifier},
                                           {"grant_type", "authorization_code"},
                                           {"redirect_uri", kReplyUrl},
                                           {"scope", kScope}},
                              "Microsoft token exchange", "Microsoft rejected the sign-in code");
    }

    OAuthTokens refresh_oauth(const std::string &refresh_token) {
        return exchange_oauth(cpr::Payload{{"client_id", kClientId},
                                           {"refresh_token", refresh_token},
                                           {"grant_type", "refresh_token"},
                                           {"redirect_uri", kReplyUrl},
                                           {"scope", kScope}},
                              "Microsoft token refresh", "Microsoft rejected the token refresh");
    }

    SisuAuthorization sisu_authorize(const std::string &session_id, const std::string &access_token,
                                     const std::string &device_token, const ProofKey &key) {
        const json body = {{"AccessToken", "t=" + access_token},
                           {"AppId", kClientId},
                           {"DeviceToken", device_token},
                           {"ProofKey", proof_jwk(key)},
                           {"Sandbox", "RETAIL"},
                           {"SessionId", session_id.empty() ? json(nullptr) : json(session_id)},
                           {"SiteName", "user.auth.xboxlive.com"},
                           {"RelyingParty", "http://xboxlive.com"},
                           {"UseModernGamertag", true}};
        const auto response = signed_post(kSisuAuthorizeUrl, "/authorize", body, key, false);
        const auto doc = parse_body(response, "Xbox authorization");
        return SisuAuthorization{.user_token = nested_token(doc, "UserToken", "Xbox authorization"),
                                 .title_token = nested_token(doc, "TitleToken", "Xbox authorization")};
    }

    XstsToken xsts_authorize(const SisuAuthorization &authorization, const std::string &device_token,
                             const ProofKey &key) {
        const json body = {{"RelyingParty", "rp://api.minecraftservices.com/"},
                           {"TokenType", "JWT"},
                           {"Properties",
                            {{"SandboxId", "RETAIL"},
                             {"UserTokens", json::array({authorization.user_token})},
                             {"DeviceToken", device_token},
                             {"TitleToken", authorization.title_token}}}};
        const auto response = signed_post(kXstsUrl, "/xsts/authorize", body, key, true);
        if (response.status_code == 401) {
            const auto doc = json::parse(response.text, nullptr, false);
            throw std::runtime_error(xsts_error_message(doc.is_object() ? doc.value("XErr", 0LL) : 0LL));
        }
        const auto doc = parse_body(response, "Xbox XSTS authorization");
        std::string user_hash;
        try {
            user_hash = doc.at("DisplayClaims").at("xui").at(0).at("uhs").get<std::string>();
        } catch (const json::exception &) {
            throw std::runtime_error("Xbox XSTS response is missing the user hash");
        }
        return XstsToken{.token = require_string(doc, "Token", "Xbox XSTS authorization"),
                         .user_hash = user_hash};
    }

    std::string launcher_login(const XstsToken &xsts) {
        const json body = {{"platform", "PC_LAUNCHER"},
                           {"xtoken", fmt::format("XBL3.0 x={};{}", xsts.user_hash, xsts.token)}};
        const auto response = cpr::Post(cpr::Url{kLauncherLoginUrl}, cpr::Body{body.dump()},
                                        cpr::Header{{"Content-Type", "application/json"},
                                                    {"Accept", "application/json"},
                                                    {"User-Agent", kUserAgent}});
        if (response.status_code != 200) {
            throw std::runtime_error(fmt::format("Minecraft services sign-in failed (HTTP {})",
                                                 response.status_code));
        }
        return require_string(parse_body(response, "Minecraft services"), "access_token", "Minecraft services");
    }

    MinecraftProfile minecraft_profile(const std::string &minecraft_access_token) {
        const auto response = cpr::Get(cpr::Url{kProfileUrl},
                                       cpr::Header{{"Authorization", "Bearer " + minecraft_access_token},
                                                   {"Accept", "application/json"},
                                                   {"User-Agent", kUserAgent}});
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
