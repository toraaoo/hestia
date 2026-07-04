#pragma once

#include <chrono>
#include <cstdint>
#include <filesystem>
#include <optional>
#include <string>
#include <tuple>
#include <type_traits>

#include <nlohmann/json.hpp>

#include <hestia/ipc/protocol.h>

// Conventions for the typed wire contracts. A call contract names its channel
// once (kChannel) and pairs it with the Params/Result payload shapes; an event
// contract names its topic (kTopic) and is its own payload. A payload struct
// declares its wire format once as a kFields table; the generic codec below
// serves every such type, so both sides of the socket marshal through one
// definition and cannot drift.
namespace hestia::proto {
    // Field flags: how a field crosses the wire.
    inline constexpr unsigned kRequired = 1U;    // decoding a missing key throws (→ bad_request)
    inline constexpr unsigned kOmitIfEmpty = 2U; // encoding skips an empty string/path
    inline constexpr unsigned kFlatten = 4U;     // nested struct (de)serializes at the parent's level

    template <typename Owner, typename T>
    struct Field {
        const char *key;
        T Owner::*member;
        unsigned flags;
    };

    template <typename Owner, typename T>
    constexpr Field<Owner, T> field(const char *key, T Owner::*member, unsigned flags = 0) {
        return {key, member, flags};
    }

    template <typename... F>
    constexpr auto fields(F... field_list) {
        return std::make_tuple(field_list...);
    }

    template <typename T>
    concept Reflected = requires { T::kFields; };

    struct Empty {
        static constexpr auto kFields = fields();
    };

    namespace detail {
        template <typename T>
        struct is_optional : std::false_type {};
        template <typename T>
        struct is_optional<std::optional<T>> : std::true_type {};

        // Scalar bridging: paths cross as strings, durations as milliseconds;
        // everything else round-trips through nlohmann's own conversions (which
        // find the generic codec below for nested reflected types).
        template <typename T>
        void write_value(nlohmann::json &node, const T &value) {
            if constexpr (std::is_same_v<T, std::filesystem::path>) {
                node = value.string();
            } else if constexpr (std::is_same_v<T, std::chrono::milliseconds>) {
                node = static_cast<std::int64_t>(value.count());
            } else {
                node = value;
            }
        }

        template <typename T>
        void read_value(const nlohmann::json &node, T &value) {
            if constexpr (std::is_same_v<T, std::filesystem::path>) {
                value = node.get<std::string>();
            } else if constexpr (std::is_same_v<T, std::chrono::milliseconds>) {
                value = std::chrono::milliseconds(node.get<std::int64_t>());
            } else {
                value = node.get<T>();
            }
        }

        template <typename Owner, typename T>
        void write_field(nlohmann::json &j, const Field<Owner, T> &f, const Owner &owner) {
            const T &value = owner.*(f.member);
            if constexpr (is_optional<T>::value) {
                if (value) write_value(j[f.key], *value);
            } else {
                if constexpr (Reflected<T>) {
                    if ((f.flags & kFlatten) != 0) {
                        j.update(nlohmann::json(value));
                        return;
                    }
                }
                if constexpr (std::is_same_v<T, std::string> || std::is_same_v<T, std::filesystem::path>) {
                    if ((f.flags & kOmitIfEmpty) != 0 && value.empty()) return;
                }
                write_value(j[f.key], value);
            }
        }

        template <typename Owner, typename T>
        void read_field(const nlohmann::json &j, const Field<Owner, T> &f, Owner &owner) {
            T &value = owner.*(f.member);
            if constexpr (is_optional<T>::value) {
                if (const auto it = j.find(f.key); it != j.end() && !it->is_null()) {
                    typename T::value_type inner{};
                    read_value(*it, inner);
                    value = std::move(inner);
                }
            } else {
                if constexpr (Reflected<T>) {
                    if ((f.flags & kFlatten) != 0) {
                        value = j.get<T>();
                        return;
                    }
                }
                if ((f.flags & kRequired) != 0) {
                    read_value(j.at(f.key), value);
                    return;
                }
                // A missing key keeps the member's default.
                if (const auto it = j.find(f.key); it != j.end()) read_value(*it, value);
            }
        }
    } // namespace detail

    template <Reflected T>
    void to_json(nlohmann::json &j, const T &value) {
        j = nlohmann::json::object();
        std::apply([&](const auto &...f) { (detail::write_field(j, f, value), ...); }, T::kFields);
    }

    template <Reflected T>
    void from_json(const nlohmann::json &j, T &value) {
        std::apply([&](const auto &...f) { (detail::read_field(j, f, value), ...); }, T::kFields);
    }

    template <typename E>
    ipc::Event make_event(const E &event) {
        return ipc::Event{.topic = E::kTopic, .payload = event};
    }
} // namespace hestia::proto
