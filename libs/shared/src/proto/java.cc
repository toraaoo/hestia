#include "hestia/proto/java.h"

namespace hestia::proto {
    std::optional<JavaInstallPhase> parse_java_install_phase(std::string_view name) {
        if (name == "resolving") return JavaInstallPhase::resolving;
        if (name == "downloading") return JavaInstallPhase::downloading;
        if (name == "extracting") return JavaInstallPhase::extracting;
        return std::nullopt;
    }

    const char *to_string(JavaInstallPhase phase) {
        switch (phase) {
            case JavaInstallPhase::resolving: return "resolving";
            case JavaInstallPhase::downloading: return "downloading";
            case JavaInstallPhase::extracting: return "extracting";
        }
        return "resolving";
    }

    void to_json(nlohmann::json &j, JavaInstallPhase phase) {
        j = to_string(phase);
    }

    void from_json(const nlohmann::json &j, JavaInstallPhase &phase) {
        phase = parse_java_install_phase(j.get<std::string>()).value_or(JavaInstallPhase::resolving);
    }
} // namespace hestia::proto
