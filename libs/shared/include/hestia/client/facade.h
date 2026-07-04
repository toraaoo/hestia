#pragma once

// Base for the client SDK's domain facades: one non-owning handle to the
// Session core (owned by Client, whose lifetime encloses every facade).
namespace hestia::client {
    class Session;

    class Facade {
    public:
        explicit Facade(Session &session) : session_(&session) {}

    protected:
        Session *session_;
    };
} // namespace hestia::client
