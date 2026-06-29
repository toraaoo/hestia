#include "hestia/ipc/transport.h"

#if !defined(_WIN32)

#include <arpa/inet.h>
#include <atomic>
#include <cerrno>
#include <cstdint>
#include <cstring>
#include <poll.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <system_error>
#include <unistd.h>

namespace hestia::ipc {
    namespace fs = std::filesystem;

    namespace {
        // Cap frame size so a desynced peer fails fast instead of making us
        // allocate gigabytes from a bogus length prefix.
        constexpr std::uint32_t kMaxFrame = 16u * 1024 * 1024;

        [[noreturn]] void throw_errno(const char *what) {
            throw std::system_error(errno, std::generic_category(), what);
        }

        bool read_all(int fd, void *buf, std::size_t n) {
            auto *p = static_cast<char *>(buf);
            for (std::size_t got = 0; got < n;) {
                const ssize_t r = ::read(fd, p + got, n - got);
                if (r > 0) { got += static_cast<std::size_t>(r); continue; }
                if (r < 0 && errno == EINTR) continue;
                return false; // EOF or error
            }
            return true;
        }

        bool write_all(int fd, const void *buf, std::size_t n) {
            const auto *p = static_cast<const char *>(buf);
            for (std::size_t sent = 0; sent < n;) {
                const ssize_t r = ::write(fd, p + sent, n - sent);
                if (r >= 0) { sent += static_cast<std::size_t>(r); continue; }
                if (errno == EINTR) continue;
                return false;
            }
            return true;
        }

        bool read_frame(int fd, std::string &out) {
            std::uint32_t len_be = 0;
            if (!read_all(fd, &len_be, sizeof(len_be))) return false;
            const std::uint32_t len = ntohl(len_be);
            if (len > kMaxFrame) return false;
            out.resize(len);
            return len == 0 || read_all(fd, out.data(), len);
        }

        bool write_frame(int fd, std::string_view msg) {
            const std::uint32_t len_be = htonl(static_cast<std::uint32_t>(msg.size()));
            if (!write_all(fd, &len_be, sizeof(len_be))) return false;
            return write_all(fd, msg.data(), msg.size());
        }

        // Fill a sockaddr_un from a filesystem path, guarding the fixed-size
        // sun_path buffer (~108 bytes — the classic Unix-socket trap).
        void fill_addr(sockaddr_un &addr, const std::string &path) {
            if (path.size() >= sizeof(addr.sun_path)) {
                throw std::system_error(ENAMETOOLONG, std::generic_category(),
                                        "socket path too long: " + path);
            }
            addr.sun_family = AF_UNIX;
            std::memcpy(addr.sun_path, path.c_str(), path.size() + 1);
        }

        // Is a daemon actually answering on `path`? Used to tell a live daemon
        // (refuse to start) from a stale socket left by a crash (reclaim it).
        bool endpoint_alive(const std::string &path) {
            const int fd = ::socket(AF_UNIX, SOCK_STREAM, 0);
            if (fd < 0) return false;
            sockaddr_un addr{};
            try { fill_addr(addr, path); } catch (...) { ::close(fd); return false; }
            const bool ok = ::connect(fd, reinterpret_cast<sockaddr *>(&addr), sizeof(addr)) == 0;
            ::close(fd);
            return ok;
        }

        class PosixListener final : public Listener {
        public:
            PosixListener(int fd, fs::path path) : fd_(fd), path_(std::move(path)) {
                if (::pipe(stop_pipe_) != 0) {
                    ::close(fd_);
                    throw_errno("pipe");
                }
            }

            ~PosixListener() override {
                if (fd_ >= 0) ::close(fd_);
                if (stop_pipe_[0] >= 0) ::close(stop_pipe_[0]);
                if (stop_pipe_[1] >= 0) ::close(stop_pipe_[1]);
                std::error_code ec;
                fs::remove(path_, ec); // best-effort cleanup of our own socket
            }

            void serve(const RequestHandler &handler) override {
                running_ = true;
                while (running_) {
                    pollfd fds[2] = {
                        {fd_, POLLIN, 0},
                        {stop_pipe_[0], POLLIN, 0},
                    };
                    const int n = ::poll(fds, 2, -1);
                    if (n < 0) {
                        if (errno == EINTR) continue;
                        break;
                    }
                    if (fds[1].revents & POLLIN) break; // stop() was called
                    if (!(fds[0].revents & POLLIN)) continue;

                    const int conn = ::accept(fd_, nullptr, nullptr);
                    if (conn < 0) continue;
                    // Phase 1: one request/response per connection. The persistent,
                    // multiplexed connection for the event stream arrives in Phase 3.
                    std::string request;
                    if (read_frame(conn, request)) {
                        const std::string response = handler(request);
                        write_frame(conn, response);
                    }
                    ::close(conn);
                }
                running_ = false;
            }

            void stop() override {
                running_ = false;
                const char byte = 1;
                // write() is async-signal-safe; this is the only thing the signal
                // handler touches.
                const ssize_t ignored = ::write(stop_pipe_[1], &byte, 1);
                (void) ignored;
            }

        private:
            int fd_ = -1;
            int stop_pipe_[2] = {-1, -1};
            fs::path path_;
            std::atomic<bool> running_{false};
        };

        class PosixChannel final : public Channel {
        public:
            explicit PosixChannel(fs::path path) : path_(std::move(path)) {}

            std::string send(std::string_view request) override {
                // Dial per request: matches the server's one-request-per-connection
                // model in Phase 1 and avoids a half-open socket between calls.
                const int fd = ::socket(AF_UNIX, SOCK_STREAM, 0);
                if (fd < 0) throw_errno("socket");
                sockaddr_un addr{};
                fill_addr(addr, path_.string());
                if (::connect(fd, reinterpret_cast<sockaddr *>(&addr), sizeof(addr)) != 0) {
                    ::close(fd);
                    throw_errno("connect");
                }
                std::string response;
                const bool ok = write_frame(fd, request) && read_frame(fd, response);
                ::close(fd);
                if (!ok) {
                    throw std::system_error(EIO, std::generic_category(),
                                            "daemon closed the connection");
                }
                return response;
            }

        private:
            fs::path path_;
        };
    } // namespace

    std::unique_ptr<Listener> bind_listener(const fs::path &endpoint) {
        std::error_code ec;
        fs::create_directories(endpoint.parent_path(), ec);

        const int fd = ::socket(AF_UNIX, SOCK_STREAM, 0);
        if (fd < 0) throw_errno("socket");

        sockaddr_un addr{};
        const std::string path = endpoint.string();
        try {
            fill_addr(addr, path);
        } catch (...) {
            ::close(fd);
            throw;
        }

        auto try_bind = [&] {
            return ::bind(fd, reinterpret_cast<sockaddr *>(&addr), sizeof(addr)) == 0;
        };

        if (!try_bind()) {
            if (errno != EADDRINUSE) {
                ::close(fd);
                throw_errno("bind");
            }
            // Address in use: a live daemon, or a stale socket from a crash?
            if (endpoint_alive(path)) {
                ::close(fd);
                throw std::system_error(EADDRINUSE, std::generic_category(),
                                        "hestiad is already running");
            }
            ::unlink(path.c_str()); // reclaim the stale socket
            if (!try_bind()) {
                ::close(fd);
                throw_errno("bind");
            }
        }

        if (::listen(fd, 16) != 0) {
            ::close(fd);
            throw_errno("listen");
        }
        return std::make_unique<PosixListener>(fd, endpoint);
    }

    std::unique_ptr<Channel> connect(const fs::path &endpoint) {
        // Fail fast here if nothing is listening, so callers get the error at
        // connect() rather than on the first send().
        if (!endpoint_alive(endpoint.string())) {
            throw std::system_error(ENOENT, std::generic_category(),
                                    "no daemon at " + endpoint.string());
        }
        return std::make_unique<PosixChannel>(endpoint);
    }
}

#endif // !_WIN32
