#include "hestia/proto/accounts.h"

#include <stdexcept>

namespace hestia::proto {
    std::optional<LoginMethod> parse_login_method(std::string_view name) {
        if (name == "device_code") return LoginMethod::device_code;
        if (name == "sisu") return LoginMethod::sisu;
        return std::nullopt;
    }

    const char *to_string(LoginMethod method) {
        return method == LoginMethod::sisu ? "sisu" : "device_code";
    }

    void to_json(nlohmann::json &j, LoginMethod method) {
        j = to_string(method);
    }

    void from_json(const nlohmann::json &j, LoginMethod &method) {
        const auto name = j.get<std::string>();
        const auto parsed = parse_login_method(name);
        if (!parsed) {
            throw std::runtime_error("unknown login method: " + name);
        }
        method = *parsed;
    }
} // namespace hestia::proto
