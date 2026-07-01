#include <gtest/gtest.h>

#include <atomic>
#include <filesystem>
#include <memory>
#include <optional>
#include <string>
#include <system_error>
#include <thread>

#include <hestia/ipc/transport.h>

#if !defined(_WIN32)
#include <unistd.h>
#endif

namespace fs = std::filesystem;
using namespace hestia::ipc;

namespace {
    // A unique endpoint per test so parallel cases never share a socket/pipe.
    fs::path unique_endpoint() {
        static std::atomic<int> counter{0};
        const int n = counter.fetch_add(1);
#if defined(_WIN32)
        return fs::path(R"(\\.\pipe\hestia-test-)" + std::to_string(n));
#else
        return fs::temp_directory_path() /
               ("hestia-test-" + std::to_string(::getpid()) + "-" + std::to_string(n) + ".sock");
#endif
    }

    // A listener serving on its own thread with an echo handler, torn down on
    // destruction so a failed assertion can never leak the serve thread.
    struct EchoServer {
        std::unique_ptr<Listener> listener;
        std::thread thread;
        std::atomic<bool> saw_peer{false};
        std::atomic<std::uint32_t> peer_uid{0};

        explicit EchoServer(const fs::path &endpoint) : listener(bind_listener(endpoint)) {
            thread = std::thread([this] {
                listener->serve([this](std::shared_ptr<Connection> conn, const Peer &peer) {
                    saw_peer = peer.local;
                    peer_uid = peer.uid;
                    while (auto frame = conn->recv()) {
                        conn->send(*frame);
                    }
                });
            });
        }

        ~EchoServer() {
            listener->stop();
            if (thread.joinable()) thread.join();
        }
    };
}

TEST(Transport, RoundTripsFramesIncludingBinaryPayloads) {
    const fs::path endpoint = unique_endpoint();
    EchoServer server(endpoint);

    auto client = connect(endpoint);
    ASSERT_NE(client, nullptr);

    // Frames are opaque bytes: embedded NULs and non-JSON content must survive.
    const std::string binary("head\0\x01\x02tail", 12);
    ASSERT_TRUE(client->send(binary));
    const auto echoed = client->recv();
    ASSERT_TRUE(echoed.has_value());
    EXPECT_EQ(*echoed, binary);

    // The connection is a persistent, multiplexed pipe — many frames each way.
    for (int i = 0; i < 100; ++i) {
        const std::string frame = "frame-" + std::to_string(i);
        ASSERT_TRUE(client->send(frame));
        const auto back = client->recv();
        ASSERT_TRUE(back.has_value());
        EXPECT_EQ(*back, frame);
    }

    client->close();
}

TEST(Transport, RecvReturnsNulloptWhenPeerCloses) {
    const fs::path endpoint = unique_endpoint();
    EchoServer server(endpoint);

    auto client = connect(endpoint);
    ASSERT_NE(client, nullptr);
    client->close();
    // Once closed, recv() unblocks with nullopt rather than hanging.
    EXPECT_FALSE(client->recv().has_value());
}

#if !defined(_WIN32)
TEST(Transport, HandlerReceivesLocalPeerIdentity) {
    const fs::path endpoint = unique_endpoint();
    EchoServer server(endpoint);

    auto client = connect(endpoint);
    ASSERT_NE(client, nullptr);
    ASSERT_TRUE(client->send("ping"));
    ASSERT_TRUE(client->recv().has_value()); // handler has run by now

    EXPECT_TRUE(server.saw_peer.load());
    EXPECT_EQ(server.peer_uid.load(), static_cast<std::uint32_t>(::getuid()));

    client->close();
}
#endif

TEST(Transport, SecondBindOnSameEndpointIsRefused) {
    const fs::path endpoint = unique_endpoint();
    EchoServer server(endpoint);

    // Single-instance guard: a live daemon already owns the endpoint.
    EXPECT_THROW(bind_listener(endpoint), std::system_error);
}

TEST(Transport, ConnectWithoutDaemonThrows) {
    const fs::path endpoint = unique_endpoint();
    // Nothing is listening, so there is no daemon to reach.
    EXPECT_THROW(connect(endpoint), std::system_error);
}
