#include "session.h"

#include <atomic>
#include <chrono>
#include <stdexcept>
#include <utility>

#include <spdlog/spdlog.h>

#include <hestia/proto/events.h>

#if !defined(_WIN32)
#include <unistd.h>
#else
#include <windows.h>
#endif

namespace hestia::client {
    using nlohmann::json;

    ipc::Response must(ipc::Response res) {
        if (!res.ok) {
            throw std::runtime_error(res.error ? res.error->code + ": " + res.error->message : "daemon error");
        }
        return res;
    }

    std::string job_id(const char *prefix) {
        static std::atomic<int> counter{0};
#if defined(_WIN32)
        const auto pid = static_cast<long long>(::GetCurrentProcessId());
#else
        const auto pid = static_cast<long long>(::getpid());
#endif
        return std::string(prefix) + "-" + std::to_string(pid) + "-" + std::to_string(++counter);
    }

    Session::Session(std::shared_ptr<ipc::Connection> connection) : conn_(std::move(connection)) {
        reader_ = std::thread([this] { read_loop(); });
    }

    Session::~Session() {
        if (conn_) conn_->close();
        if (reader_.joinable()) reader_.join();
    }

    void Session::read_loop() {
        while (auto frame = conn_->recv()) {
            json j;
            try {
                j = json::parse(*frame);
            } catch (...) {
                continue; // ignore a malformed frame rather than tear down
            }
            if (ipc::is_event(j)) {
                EventCallback cb;
                {
                    std::scoped_lock const lk(mu_);
                    cb = on_event_;
                }
                if (cb) cb(ipc::decode_event(j));
                continue;
            }
            ipc::Response res = ipc::decode_response(j);
            const long long id = res.id.value_or(0);
            std::scoped_lock const lk(mu_);
            ready_[id] = std::move(res);
            cv_.notify_all();
        }
        // The connection closed: wake every waiter so they fail instead of
        // blocking forever.
        std::scoped_lock const lk(mu_);
        closed_ = true;
        cv_.notify_all();
    }

    ipc::Response Session::call_raw(const std::string &channel, json payload, std::chrono::milliseconds timeout) {
        long long id;
        {
            std::scoped_lock const lk(mu_);
            if (closed_) throw std::runtime_error("daemon connection lost");
            id = next_id_++;
        }
        ipc::Request req;
        req.channel = channel;
        req.payload = std::move(payload);
        req.id = id;
        spdlog::debug("call {} (id {})", channel, id);
        if (!conn_->send(ipc::encode(req))) {
            throw std::runtime_error("daemon connection lost");
        }

        std::unique_lock<std::mutex> lk(mu_);
        // Bound the wait so a wedged handler can't hang the caller forever.
        if (!cv_.wait_for(lk, timeout, [&] { return closed_ || ready_.count(id) > 0; })) {
            ready_.erase(id);
            spdlog::warn("call {} (id {}) timed out", channel, id);
            throw std::runtime_error("timed out waiting for daemon response on '" + channel + "'");
        }
        const auto it = ready_.find(id);
        if (it == ready_.end()) throw std::runtime_error("daemon closed the connection");
        ipc::Response res = std::move(it->second);
        ready_.erase(it);
        spdlog::debug("call {} (id {}) -> {}", channel, id, res.ok ? "ok" : "error");
        return res;
    }

    void Session::set_event_callback(EventCallback cb) {
        std::scoped_lock const lk(mu_);
        on_event_ = std::move(cb);
    }

    bool Session::is_closed() {
        std::scoped_lock const lk(mu_);
        return closed_;
    }

    json Session::run_job(const std::string &id, const char *done_topic, const char *error_topic,
                          const std::function<void(const ipc::Event &)> &on_event,
                          const std::function<void()> &start) {
        struct Outcome {
            std::mutex mu;
            std::condition_variable cv;
            bool done = false;
            bool ok = false;
            std::string message;
            json payload;
        };
        const auto outcome = std::make_shared<Outcome>();

        set_event_callback([outcome, id, done_topic, error_topic, on_event](const ipc::Event &event) {
            if (event.payload.value("id", std::string{}) != id) return;
            if (event.topic != done_topic && event.topic != error_topic) {
                if (on_event) on_event(event);
                return;
            }
            std::scoped_lock const lk(outcome->mu);
            outcome->done = true;
            outcome->ok = event.topic == done_topic;
            outcome->message = event.payload.value("message", std::string{});
            outcome->payload = event.payload;
            outcome->cv.notify_all();
        });

        try {
            call<proto::EventsSubscribe>({.id = id});
            start();

            std::unique_lock<std::mutex> lk(outcome->mu);
            while (!outcome->done) {
                outcome->cv.wait_for(lk, std::chrono::milliseconds(500));
                if (!outcome->done && is_closed()) {
                    throw std::runtime_error("daemon connection lost");
                }
            }
        } catch (...) {
            set_event_callback({});
            throw;
        }
        set_event_callback({});
        if (!outcome->ok) throw std::runtime_error(outcome->message);
        return outcome->payload;
    }
} // namespace hestia::client
