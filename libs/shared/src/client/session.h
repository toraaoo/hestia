#pragma once

#include <condition_variable>
#include <functional>
#include <map>
#include <memory>
#include <mutex>
#include <optional>
#include <string>
#include <thread>

#include <nlohmann/json.hpp>

#include <hestia/ipc/errors.h>
#include <hestia/ipc/protocol.h>
#include <hestia/ipc/transport.h>

// The client SDK's connection core, shared by every domain facade: one
// persistent, multiplexed connection whose reader thread fulfils pending
// requests by id and delivers events to the installed callback. The typed
// call<Contract>() marshals through the contract's ADL codec, so the facades
// stay one-liners and cannot drift from the daemon.
namespace hestia::client {
    // Throw on a daemon-side error; otherwise hand the response back.
    ipc::Response must(ipc::Response res);

    // A client-generated job id lets callers subscribe before starting a job, so
    // even one that finishes instantly cannot slip its terminal event past us.
    std::string job_id(const char *prefix);

    class Session {
    public:
        using EventCallback = std::function<void(const ipc::Event &)>;

        explicit Session(std::shared_ptr<ipc::Connection> connection);
        ~Session();

        // Raw request; throws only on transport failure (a daemon-side error is a
        // Response with ok == false). The typed calls below are built on this.
        ipc::Response call_raw(const std::string &channel, nlohmann::json payload);

        // Send C::Params over C::kChannel and decode C::Result, throwing
        // std::runtime_error on a transport failure or a daemon-side error.
        template <typename C>
        typename C::Result call(const typename C::Params &params = {}) {
            return must(call_raw(C::kChannel, params)).payload.template get<typename C::Result>();
        }

        // Like call(), but a not_found error becomes nullopt instead of a throw.
        template <typename C>
        std::optional<typename C::Result> try_call(const typename C::Params &params = {}) {
            auto res = call_raw(C::kChannel, params);
            if (!res.ok && res.error && res.error->code == ipc::errors::kNotFound) return std::nullopt;
            return must(std::move(res)).payload.template get<typename C::Result>();
        }

        // Events arrive as raw envelopes; facades layer their own typing on top.
        // A single slot: installing a callback replaces the previous one.
        void set_event_callback(EventCallback cb);

        bool is_closed();

        // Subscribe to `id`'s events, invoke `start`, and block until the done or
        // error topic arrives, handing every other matching event to `on_event`.
        // Returns the done event's payload; throws the error event's message.
        nlohmann::json run_job(const std::string &id, const char *done_topic, const char *error_topic,
                               const std::function<void(const ipc::Event &)> &on_event,
                               const std::function<void()> &start);

    private:
        void read_loop();

        std::shared_ptr<ipc::Connection> conn_;
        std::thread reader_;
        std::mutex mu_;
        std::condition_variable cv_;
        long long next_id_ = 1;
        std::map<long long, ipc::Response> ready_;
        EventCallback on_event_;
        bool closed_ = false;

        static constexpr auto kCallTimeout = std::chrono::seconds(10);
    };
} // namespace hestia::client
