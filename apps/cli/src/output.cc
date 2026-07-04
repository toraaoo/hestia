#include "output.h"

#include <cstdio>
#include <iostream>
#include <sstream>

#include <ftxui/dom/elements.hpp>
#include <ftxui/screen/screen.hpp>

namespace hestia::cli {
    namespace {
        constexpr const char *kHideCursor = "\033[?25l";
        constexpr const char *kShowCursor = "\033[?25h";
    } // namespace

    std::string human_size(std::uint64_t bytes) {
        constexpr const char *units[] = {"B", "kB", "MB", "GB", "TB"};
        auto value = static_cast<double>(bytes);
        std::size_t unit = 0;
        while (value >= 1000.0 && unit + 1 < std::size(units)) {
            value /= 1000.0;
            ++unit;
        }
        char buf[32];
        if (unit == 0) {
            std::snprintf(buf, sizeof buf, "%llu%s", static_cast<unsigned long long>(bytes), units[unit]);
        } else {
            std::snprintf(buf, sizeof buf, "%.1f%s", value, units[unit]);
        }
        return buf;
    }

    void print_table(const std::vector<std::string> &headers, const std::vector<std::vector<std::string>> &rows) {
        // gridbox aligns each column to its widest cell; a trailing gap on every
        // cell but the last keeps the borderless three-space column style.
        const auto to_line = [](const std::vector<std::string> &cells) {
            ftxui::Elements line;
            line.reserve(cells.size());
            for (std::size_t i = 0; i < cells.size(); ++i) {
                line.push_back(ftxui::text(i + 1 < cells.size() ? cells[i] + "   " : cells[i]));
            }
            return line;
        };

        std::vector<ftxui::Elements> lines;
        lines.reserve(rows.size() + 1);
        lines.push_back(to_line(headers));
        for (const auto &row: rows) lines.push_back(to_line(row));

        auto element = ftxui::gridbox(lines);
        auto screen = ftxui::Screen::Create(ftxui::Dimension::Fit(element));
        ftxui::Render(screen, element);

        // FTXUI renders a fixed canvas (CRLF rows, right-padded to full width);
        // trim each line so the borderless output stays clean when piped.
        std::istringstream in(screen.ToString());
        std::string line;
        while (std::getline(in, line)) {
            while (!line.empty() && (line.back() == ' ' || line.back() == '\r')) line.pop_back();
            std::cout << line << '\n';
        }
    }

    Spinner::Spinner(std::string label) : label_(std::move(label)) {
        thread_ = std::thread([this] {
            constexpr int kBrailleCharset = 15;
            std::size_t step = 0;
            std::cerr << kHideCursor << std::flush;
            while (!stop_) {
                auto element = ftxui::hbox({ftxui::spinner(kBrailleCharset, step++), ftxui::text(" " + label_)});
                auto screen = ftxui::Screen::Create(ftxui::Dimension::Fit(element));
                ftxui::Render(screen, element);
                std::cerr << '\r' << screen.ToString() << std::flush;
                std::this_thread::sleep_for(std::chrono::milliseconds(80));
            }
            std::cerr << '\r' << std::string(label_.size() + 2, ' ') << '\r' << kShowCursor << std::flush;
        });
    }

    Spinner::~Spinner() {
        stop();
    }

    void Spinner::stop() {
        if (thread_.joinable()) {
            stop_ = true;
            thread_.join();
        }
    }

    ProgressBar::ProgressBar(std::string status, bool bytes) : status_(std::move(status)), bytes_(bytes) {}

    ProgressBar::~ProgressBar() {
        finish();
    }

    void ProgressBar::update(std::uint64_t current, std::uint64_t total) {
        if (!rendered_) std::cerr << kHideCursor;
        const auto now = std::chrono::steady_clock::now();
        if (rendered_) {
            const std::chrono::duration<double> elapsed = now - last_time_;
            if (elapsed.count() > 0 && current >= last_count_) {
                const double instant = static_cast<double>(current - last_count_) / elapsed.count();
                per_second_ = per_second_ == 0.0 ? instant : 0.7 * per_second_ + 0.3 * instant;
            }
        }
        last_time_ = now;
        last_count_ = current;
        rendered_ = true;

        const auto count = [&](std::uint64_t n) { return bytes_ ? human_size(n) : std::to_string(n); };
        std::string tail = "  " + count(current);
        if (total > 0) tail += "/" + count(total);
        if (bytes_ && per_second_ > 0.0) {
            tail += "  " + human_size(static_cast<std::uint64_t>(per_second_)) + "/s";
        }
        tail += "    ";

        if (total == 0) {
            std::cerr << '\r' << status_ << tail << std::flush;
            return;
        }
        const auto fraction = static_cast<float>(static_cast<double>(current) / static_cast<double>(total));
        auto element = ftxui::hbox({
            ftxui::text(status_ + " "),
            ftxui::gauge(fraction) | ftxui::size(ftxui::WIDTH, ftxui::EQUAL, 40),
            ftxui::text(tail),
        });
        auto screen = ftxui::Screen::Create(ftxui::Dimension::Fit(element));
        ftxui::Render(screen, element);
        std::cerr << '\r' << screen.ToString() << std::flush;
    }

    void ProgressBar::finish() {
        if (rendered_) {
            std::cerr << kShowCursor << '\n';
            rendered_ = false;
        }
    }
} // namespace hestia::cli
