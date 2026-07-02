#include <gtest/gtest.h>

#include <hestia/engine/greeting/greeting.h>

using hestia::greeting::greet;

TEST(Greeting, UsesTheName) {
    EXPECT_EQ(greet("Ada"), "Hello, Ada!");
}

TEST(Greeting, EmptyFallsBackToGeneric) {
    EXPECT_EQ(greet(), "Hello there!");
    EXPECT_FALSE(greet().empty());
}
