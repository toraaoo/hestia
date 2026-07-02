#include "commands/download_command.h"

#include <filesystem>
#include <iostream>
#include <string>

#include <hestia/client/client.h>
#include <hestia/ipc/download.h>

namespace hestia::cli {
    namespace {
        // "<algorithm>:<hex>" → the request's checksum fields; empty return =
        // malformed (the daemon never sees a request we can reject here).
        bool parse_checksum(const std::string &value, client::DownloadRequest &request) {
            const auto sep = value.find(':');
            if (sep == std::string::npos) return false;
            const std::string algorithm = value.substr(0, sep);
            const std::string hex = value.substr(sep + 1);
            const auto parsed = ipc::parse_hash_algorithm(algorithm);
            if (!parsed || !ipc::is_valid_checksum(ipc::Checksum{*parsed, hex})) {
                return false;
            }
            request.checksum_algorithm = algorithm;
            request.checksum_hex = hex;
            return true;
        }

        void print_progress(const client::DownloadProgress &progress) {
            std::cerr << "\rdownloading ";
            if (progress.total > 0) {
                std::cerr << progress.downloaded * 100 / progress.total << "% (" << progress.downloaded << "/"
                          << progress.total << " bytes)";
            } else {
                std::cerr << progress.downloaded << " bytes";
            }
            std::cerr << std::flush;
        }
    } // namespace

    void DownloadCommand::register_command(CLI::App &parent, AppContext &ctx) {
        auto *cmd = parent.add_subcommand("download", "Download a file via the daemon");
        cmd->add_option("url", url_, "URL to download")->required();
        cmd->add_option("dest", dest_, "Destination file path")->required();
        cmd->add_option("--checksum", checksum_, "Expected checksum as <algorithm>:<hex> (sha1 or sha256)");
        cmd->callback([this, &ctx] {
            client::DownloadRequest request;
            request.url = url_;
            request.destination = std::filesystem::absolute(dest_).string();
            if (!checksum_.empty() && !parse_checksum(checksum_, request)) {
                std::cerr << "hestia: --checksum must be sha1:<40 hex chars> or "
                             "sha256:<64 hex chars>\n";
                ctx.exit_code = 1;
                return;
            }
            ctx.with_client([&request](client::Client &client) {
                bool progressed = false;
                try {
                    client.download(request, [&progressed](const client::DownloadProgress &p) {
                        progressed = true;
                        print_progress(p);
                    });
                } catch (...) {
                    if (progressed) std::cerr << '\n';
                    throw;
                }
                if (progressed) std::cerr << '\n';
                std::cout << "downloaded to " << request.destination << '\n';
            });
        });
    }
} // namespace hestia::cli
