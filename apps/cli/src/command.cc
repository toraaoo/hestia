#include "command.h"

#include <iostream>

#include <hestia/client.h>

namespace hestia::cli {
    void AppContext::with_client(const std::function<void(client::Client &)> &body) {
        try {
            auto client = client::Client::connect();
            body(client);
        } catch (const std::exception &e) {
            std::cerr << "hestia: " << e.what() << '\n';
            exit_code = 1;
        }
    }
} // namespace hestia::cli
