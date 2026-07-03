#include "java/extract.h"

#include <stdexcept>
#include <string>
#include <string_view>
#include <vector>

#include <fmt/format.h>

#if defined(_WIN32)
#include <windows.h>
#else
#include <sys/wait.h>
#include <unistd.h>
#endif

namespace hestia::engine {
    namespace fs = std::filesystem;

    namespace {
        using LineCallback = std::function<void(std::string_view)>;

        void feed_lines(std::string &pending, const LineCallback &on_line) {
            std::size_t pos;
            while ((pos = pending.find('\n')) != std::string::npos) {
                if (on_line) on_line(std::string_view(pending).substr(0, pos));
                pending.erase(0, pos + 1);
            }
        }

#if defined(_WIN32)
        int run_tar(const std::vector<fs::path> &args, const LineCallback &on_line) {
            SECURITY_ATTRIBUTES sa{.nLength = sizeof(SECURITY_ATTRIBUTES), .bInheritHandle = TRUE};
            HANDLE read_end = nullptr;
            HANDLE write_end = nullptr;
            if (!::CreatePipe(&read_end, &write_end, &sa, 0)) return -1;
            ::SetHandleInformation(read_end, HANDLE_FLAG_INHERIT, 0);

            std::wstring cmd = L"tar.exe";
            for (const auto &arg: args) {
                cmd += L" \"" + arg.wstring() + L"\"";
            }

            STARTUPINFOW si{};
            si.cb = sizeof(si);
            si.dwFlags = STARTF_USESTDHANDLES;
            si.hStdOutput = write_end;
            si.hStdError = write_end;
            PROCESS_INFORMATION pi{};
            const BOOL ok = ::CreateProcessW(nullptr, cmd.data(), nullptr, nullptr, TRUE, CREATE_NO_WINDOW, nullptr,
                                             nullptr, &si, &pi);
            ::CloseHandle(write_end);
            if (!ok) {
                ::CloseHandle(read_end);
                return 127;
            }

            std::string pending;
            char buf[4096];
            DWORD n = 0;
            while (::ReadFile(read_end, buf, sizeof buf, &n, nullptr) && n > 0) {
                pending.append(buf, n);
                feed_lines(pending, on_line);
            }
            ::CloseHandle(read_end);
            if (!pending.empty() && on_line) on_line(pending);

            ::WaitForSingleObject(pi.hProcess, INFINITE);
            DWORD code = 1;
            ::GetExitCodeProcess(pi.hProcess, &code);
            ::CloseHandle(pi.hThread);
            ::CloseHandle(pi.hProcess);
            return static_cast<int>(code);
        }
#else
        int run_tar(const std::vector<fs::path> &args, const LineCallback &on_line) {
            int fds[2];
            if (::pipe(fds) != 0) return -1;
            const pid_t pid = ::fork();
            if (pid < 0) {
                ::close(fds[0]);
                ::close(fds[1]);
                return -1;
            }
            if (pid == 0) {
                ::dup2(fds[1], 1);
                ::dup2(fds[1], 2);
                ::close(fds[0]);
                ::close(fds[1]);
                std::vector<std::string> storage;
                storage.reserve(args.size());
                for (const auto &arg: args) storage.push_back(arg.string());
                std::vector<char *> argv;
                argv.push_back(const_cast<char *>("tar"));
                for (auto &arg: storage) argv.push_back(arg.data());
                argv.push_back(nullptr);
                ::execvp("tar", argv.data());
                _exit(127);
            }
            ::close(fds[1]);

            std::string pending;
            char buf[4096];
            ssize_t n;
            while ((n = ::read(fds[0], buf, sizeof buf)) > 0) {
                pending.append(buf, static_cast<std::size_t>(n));
                feed_lines(pending, on_line);
            }
            ::close(fds[0]);
            if (!pending.empty() && on_line) on_line(pending);

            int status = 0;
            if (::waitpid(pid, &status, 0) < 0) return -1;
            return WIFEXITED(status) ? WEXITSTATUS(status) : -1;
        }
#endif
    } // namespace

    void extract_archive(const fs::path &archive, const fs::path &dest, const ExtractProgressCallback &on_progress) {
        fs::create_directories(dest);

        std::uint64_t total = 0;
        if (on_progress) {
            const int list_code = run_tar({"-tf", archive}, [&](std::string_view) { ++total; });
            if (list_code == 127) {
                throw std::runtime_error("cannot extract archive: the system 'tar' tool was not found");
            }
            if (list_code != 0) total = 0;
        }

        std::uint64_t done = 0;
        const int code = run_tar({"-xvf", archive, "-C", dest}, [&](std::string_view) {
            ++done;
            if (on_progress) on_progress(done, total);
        });
        if (code == 127) {
            throw std::runtime_error("cannot extract archive: the system 'tar' tool was not found");
        }
        if (code != 0) {
            throw std::runtime_error(
                fmt::format("extracting {} failed: tar exited with code {}", archive.string(), code));
        }
    }
} // namespace hestia::engine
