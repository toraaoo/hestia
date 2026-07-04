#include <hestia/engine/cache.h>

#include <atomic>
#include <cctype>
#include <string>
#include <system_error>
#include <utility>

#include <spdlog/spdlog.h>

namespace hestia::engine {
    namespace fs = std::filesystem;

    namespace {
        std::string lower_hex(const std::string &hex) {
            std::string out = hex;
            for (auto &c: out) c = static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
            return out;
        }

        fs::path blob_path(const fs::path &dir, const proto::Checksum &checksum) {
            const std::string hex = lower_hex(checksum.hex);
            return dir / proto::to_string(checksum.algorithm) / hex.substr(0, 2) / hex;
        }

        void remove_quietly(const fs::path &path) {
            std::error_code ec;
            fs::remove(path, ec);
        }
    } // namespace

    Cache::Cache(fs::path dir) : dir_(std::move(dir)) {}

    fs::path Cache::dir() const {
        std::scoped_lock const lk(mu_);
        return dir_;
    }

    void Cache::reload(fs::path dir) {
        std::scoped_lock const lk(mu_);
        dir_ = std::move(dir);
    }

    std::optional<fs::path> Cache::lookup(const proto::Checksum &checksum) const {
        if (!proto::is_valid_checksum(checksum)) return std::nullopt;
        fs::path blob = blob_path(dir(), checksum);
        std::error_code ec;
        if (!fs::is_regular_file(blob, ec)) return std::nullopt;
        return blob;
    }

    void Cache::store(const fs::path &file, const proto::Checksum &checksum) {
        if (!proto::is_valid_checksum(checksum)) return;
        const fs::path blob = blob_path(dir(), checksum);
        std::error_code ec;
        if (fs::exists(blob, ec)) return;
        fs::create_directories(blob.parent_path(), ec);
        if (ec) return;

        static std::atomic<int> counter{0};
        const fs::path tmp = blob.string() + ".part" + std::to_string(++counter);
        fs::copy_file(file, tmp, fs::copy_options::overwrite_existing, ec);
        if (ec) {
            remove_quietly(tmp);
            return;
        }
        fs::rename(tmp, blob, ec);
        if (ec) {
            remove_quietly(tmp);
            return;
        }
        spdlog::debug("cached {} ({})", checksum.hex, proto::to_string(checksum.algorithm));
    }

    void Cache::evict(const proto::Checksum &checksum) {
        if (!proto::is_valid_checksum(checksum)) return;
        spdlog::debug("evicting cache blob {}", checksum.hex);
        remove_quietly(blob_path(dir(), checksum));
    }

    std::vector<CacheEntry> Cache::entries() const {
        std::vector<CacheEntry> out;
        const fs::path base = dir();
        std::error_code ec;
        for (const auto algorithm: {proto::HashAlgorithm::sha1, proto::HashAlgorithm::sha256}) {
            const fs::path root = base / proto::to_string(algorithm);
            for (fs::recursive_directory_iterator it(root, ec), end; !ec && it != end; it.increment(ec)) {
                if (!it->is_regular_file(ec)) continue;
                const proto::Checksum checksum{.algorithm = algorithm, .hex = it->path().filename().string()};
                if (!proto::is_valid_checksum(checksum)) continue;
                std::error_code size_ec;
                const auto size = it->file_size(size_ec);
                if (size_ec) continue;
                out.push_back(CacheEntry{.checksum = checksum, .size = size});
            }
            ec.clear();
        }
        return out;
    }

    CacheUsage Cache::usage() const {
        CacheUsage usage;
        for (const auto &entry: entries()) {
            ++usage.entries;
            usage.bytes += entry.size;
        }
        return usage;
    }

    CacheUsage Cache::clear() {
        const CacheUsage freed = usage();
        std::error_code ec;
        for (const auto algorithm: {proto::HashAlgorithm::sha1, proto::HashAlgorithm::sha256}) {
            fs::remove_all(dir() / proto::to_string(algorithm), ec);
        }
        spdlog::info("cleared cache: {} entries, {} bytes freed", freed.entries, freed.bytes);
        return freed;
    }
} // namespace hestia::engine
