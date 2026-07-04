#include "accounts/login_manager.h"

#include <exception>
#include <stdexcept>
#include <utility>

#include <hestia/engine/accounts.h>
#include <hestia/proto/accounts.h>

namespace hestia::daemon {
    LoginManager::LoginManager(engine::Engine &engine, EventSink sink) : engine_(engine), sink_(std::move(sink)) {}

    LoginManager::~LoginManager() {
        cancel_.store(true);
        if (worker_.joinable()) worker_.join();
    }

    std::optional<std::string> LoginManager::start(std::string id) {
        if (id.empty()) id = "login-" + std::to_string(next_id_++);
        std::scoped_lock const lk(mu_);
        if (active_) return std::nullopt;
        if (worker_.joinable()) worker_.join();
        active_ = true;
        worker_ = std::thread([this, id] { run(id); });
        return id;
    }

    void LoginManager::run(const std::string &id) {
        try {
            const auto client_id = engine_.config().settings().auth.msa_client_id;
            if (client_id.empty()) {
                throw std::runtime_error(
                    "no Microsoft client id is configured; register an Azure application approved for the "
                    "Minecraft APIs and run `hestia config set auth.msa_client_id <id>`");
            }
            const auto account = engine_.accounts().login(
                client_id,
                [&](const proto::AccountLoginCode &code) {
                    sink_(proto::make_event(proto::AccountLoginCodeEvent{.id = id, .code = code}));
                },
                [this] { return cancel_.load(); });
            sink_(proto::make_event(proto::AccountLoginDoneEvent{.id = id, .account = account}));
        } catch (const std::exception &e) {
            sink_(proto::make_event(proto::AccountLoginErrorEvent{.id = id, .message = e.what()}));
        }
        std::scoped_lock const lk(mu_);
        active_ = false;
    }
} // namespace hestia::daemon
