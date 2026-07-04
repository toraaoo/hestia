#pragma once

#include <atomic>
#include <chrono>
#include <cstdint>
#include <string>
#include <thread>
#include <vector>

namespace hestia::cli {
    std::string human_size(std::uint64_t bytes);

    void print_table(const std::vector<std::string> &headers, const std::vector<std::vector<std::string>> &rows);

    // Animated line on stderr while waiting on the daemon or a provider;
    // clears its line on stop()/destruction.
    class Spinner {
    public:
        explicit Spinner(std::string label);
        ~Spinner();

        void stop();

    private:
        std::string label_;
        std::atomic<bool> stop_{false};
        std::thread thread_;
    };

    class ProgressBar {
    public:
        explicit ProgressBar(std::string status, bool bytes = true);
        ~ProgressBar();

        void update(std::uint64_t current, std::uint64_t total);
        void finish();

    private:
        std::string status_;
        bool bytes_;
        std::chrono::steady_clock::time_point last_time_{};
        std::uint64_t last_count_ = 0;
        double per_second_ = 0.0;
        bool rendered_ = false;
    };
} // namespace hestia::cli
