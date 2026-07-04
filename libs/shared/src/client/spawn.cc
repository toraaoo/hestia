#include "spawn.h"

#include <chrono>
#include <exception>
#include <string>
#include <thread>

#if !defined(_WIN32)
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>
#else
#include <windows.h>
#endif

#if defined(__APPLE__)
#include <cstdint>
#include <mach-o/dyld.h>
#endif

namespace hestia::client {
    namespace fs = std::filesystem;

    namespace {
        // hestiad sits beside the current binary (CLI/daemon/tray, all in bin/) or
        // in a bin/ subdirectory (the desktop launcher, a level up); else via PATH.
        fs::path find_daemon();

#if !defined(_WIN32)
        fs::path self_dir() {
#if defined(__APPLE__)
            char buf[4096];
            std::uint32_t size = sizeof(buf);
            if (_NSGetExecutablePath(buf, &size) != 0) return {}; // buffer too small
            std::error_code ec;
            const fs::path resolved = fs::weakly_canonical(fs::path(buf), ec);
            return (ec ? fs::path(buf) : resolved).parent_path();
#else
            char buf[4096];
            const ssize_t n = ::readlink("/proc/self/exe", buf, sizeof(buf) - 1);
            if (n <= 0) return {};
            buf[n] = '\0';
            return fs::path(buf).parent_path();
#endif
        }
#else
        fs::path self_dir() {
            wchar_t buf[MAX_PATH];
            const DWORD n = ::GetModuleFileNameW(nullptr, buf, MAX_PATH);
            if (n == 0 || n == MAX_PATH) return {};
            return fs::path(std::wstring(buf, n)).parent_path();
        }
#endif

        fs::path find_daemon() {
#if defined(_WIN32)
            fs::path exe = L"hestiad.exe";
#else
            fs::path exe = "hestiad";
#endif
            const fs::path dir = self_dir();
            if (!dir.empty()) {
                std::error_code ec;
                for (const fs::path &candidate: {dir / exe, dir / "bin" / exe}) {
                    if (fs::exists(candidate, ec)) return candidate;
                }
            }
            return exe; // resolved through PATH
        }
    } // namespace

#if !defined(_WIN32)
    void spawn_daemon() {
        const std::string program = find_daemon().string();

        const pid_t pid = ::fork();
        if (pid < 0) throw std::runtime_error("failed to fork to start hestiad");
        if (pid == 0) {
            ::setsid(); // detach from the frontend's session — the daemon outlives it
            if (const int devnull = ::open("/dev/null", O_RDWR); devnull >= 0) {
                ::dup2(devnull, 0);
                ::dup2(devnull, 1);
                ::dup2(devnull, 2);
                if (devnull > 2) ::close(devnull);
            }
            ::execlp(program.c_str(), "hestiad", "serve", static_cast<char *>(nullptr));
            _exit(127); // exec failed
        }
        // Parent: the daemon does not exit, so we never reap it; it reparents
        // to init. We just wait for its socket below.
    }
#else
    void spawn_daemon() {
        const fs::path program = find_daemon();

        // Quote the program path (it may contain spaces); CreateProcessW
        // parses this as the command line when lpApplicationName is null, so a
        // bare "hestiad.exe" is resolved through PATH.
        std::wstring cmd = L"\"" + program.wstring() + L"\" serve";

        STARTUPINFOW si{};
        si.cb = sizeof(si);
        PROCESS_INFORMATION pi{};
        const std::wstring workdir = self_dir().wstring();
        // DETACHED_PROCESS: no inherited console, so the daemon outlives the
        // frontend (mirrors the POSIX setsid + double-fork detachment).
        const BOOL ok = ::CreateProcessW(nullptr, cmd.data(), nullptr, nullptr, FALSE, DETACHED_PROCESS, nullptr,
                                         workdir.empty() ? nullptr : workdir.c_str(), &si, &pi);
        if (!ok) throw std::runtime_error("failed to start hestiad");
        ::CloseHandle(pi.hThread);
        ::CloseHandle(pi.hProcess);
    }
#endif

    std::shared_ptr<ipc::Connection> connect_with_retry(const fs::path &endpoint) {
        // Poll briefly for the freshly-spawned daemon's socket to appear.
        for (int attempt = 0; attempt < 60; ++attempt) {
            try {
                return ipc::connect(endpoint);
            } catch (const std::exception &) {
                std::this_thread::sleep_for(std::chrono::milliseconds(50));
            }
        }
        return nullptr;
    }
} // namespace hestia::client
