#include <hestia/logging.h>

#include <memory>
#include <system_error>
#include <vector>

#include <spdlog/sinks/rotating_file_sink.h>
#include <spdlog/sinks/stdout_color_sinks.h>
#include <spdlog/spdlog.h>

namespace hestia {
    namespace {
        spdlog::level::level_enum to_spdlog(LogLevel level) {
            switch (level) {
            case LogLevel::trace: return spdlog::level::trace;
            case LogLevel::debug: return spdlog::level::debug;
            case LogLevel::info: return spdlog::level::info;
            case LogLevel::warn: return spdlog::level::warn;
            case LogLevel::error: return spdlog::level::err;
            case LogLevel::critical: return spdlog::level::critical;
            case LogLevel::off: return spdlog::level::off;
            }
            return spdlog::level::info;
        }

        constexpr const char *kConsolePattern = "[%H:%M:%S.%e] [%^%l%$] %v";
        constexpr const char *kFilePattern = "[%Y-%m-%d %H:%M:%S.%e] [%l] [%P:%t] %v";

        constexpr std::size_t kMaxFileBytes = std::size_t{5} * 1024 * 1024;
        constexpr std::size_t kMaxFiles = 5;
    } // namespace

    void init_logging(LogLevel level, const std::filesystem::path &file) {
        const auto min_level = to_spdlog(level);

        std::vector<spdlog::sink_ptr> sinks;

        auto console = std::make_shared<spdlog::sinks::stderr_color_sink_mt>();
        console->set_pattern(kConsolePattern);
        sinks.push_back(std::move(console));

        if (!file.empty()) {
            std::error_code ec;
            std::filesystem::create_directories(file.parent_path(), ec);
            auto rotating =
                std::make_shared<spdlog::sinks::rotating_file_sink_mt>(file.string(), kMaxFileBytes, kMaxFiles);
            rotating->set_pattern(kFilePattern);
            sinks.push_back(std::move(rotating));
        }

        auto logger = std::make_shared<spdlog::logger>("hestia", sinks.begin(), sinks.end());
        logger->set_level(min_level);
        logger->flush_on(min_level);
        spdlog::set_default_logger(std::move(logger));
    }
} // namespace hestia
