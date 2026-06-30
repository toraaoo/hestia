#include <gtest/gtest.h>

#include <exception>
#include <string>
#include <string_view>

#include <hestia/ipc/protocol.h>
#include <nlohmann/json.hpp>

using namespace hestia::ipc;

TEST(Protocol, RequestRoundTrip) {
    Request req;
    req.channel = "config.get";
    req.payload = {{"key", "theme"}};
    req.id = 42;

    const Request back = decode_request(encode(req));
    EXPECT_EQ(back.channel, "config.get");
    EXPECT_EQ(back.payload.value("key", std::string{}), "theme");
    ASSERT_TRUE(back.id.has_value());
    EXPECT_EQ(*back.id, 42);
    EXPECT_EQ(back.version, kProtocolVersion);
}

TEST(Protocol, RequestWithoutIdRoundTrips) {
    Request req;
    req.channel = "health.ping";
    const Request back = decode_request(encode(req));
    EXPECT_EQ(back.channel, "health.ping");
    EXPECT_FALSE(back.id.has_value());
}

TEST(Protocol, ResponseSuccessRoundTrip) {
    const Response ok = Response::success({{"value", "dark"}});
    const Response back = decode_response(std::string_view(encode(ok)));
    EXPECT_TRUE(back.ok);
    EXPECT_EQ(back.payload.value("value", std::string{}), "dark");
}

TEST(Protocol, ResponseFailureRoundTrip) {
    const Response err = Response::failure("bad_request", "nope");
    const Response back = decode_response(std::string_view(encode(err)));
    EXPECT_FALSE(back.ok);
    ASSERT_TRUE(back.error.has_value());
    EXPECT_EQ(back.error->code, "bad_request");
    EXPECT_EQ(back.error->message, "nope");
}

TEST(Protocol, EventRoundTrip) {
    const Event ev{"process.state", {{"id", "srv"}, {"state", "running"}}};
    const Event back = decode_event(nlohmann::json::parse(encode(ev)));
    EXPECT_EQ(back.topic, "process.state");
    EXPECT_EQ(back.payload.value("state", std::string{}), "running");
}

TEST(Protocol, EventsAreClassifiedApartFromResponses) {
    const Event ev{"process.log", {{"id", "srv"}}};
    EXPECT_TRUE(is_event(nlohmann::json::parse(encode(ev))));
    EXPECT_FALSE(is_event(nlohmann::json::parse(encode(Response::success()))));
}

TEST(Protocol, MalformedFrameThrows) {
    EXPECT_THROW(decode_request("this is not json"), std::exception);
}

TEST(Protocol, VersionCompatibility) {
    EXPECT_TRUE(compatible(kProtocolVersion));
    EXPECT_FALSE(compatible(kProtocolVersion + 1));
}
