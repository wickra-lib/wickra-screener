// Streaming equals batch, through the C++ hull.
//
// The hull hides the two-call length protocol, which is exactly why it needs
// its own test: `command` asks the ABI for a length and then reads, so before
// the ABI carried a produced-but-undelivered response, every mutating command
// a C++ caller sent ran twice. The golden corpus could not see it -- it only
// ever sends {"cmd":"scan"}, a pure function of its payload.
//
// The dataset is inline rather than read from golden/, because splitting the
// corpus by hand here would test the splitter. What matters is that the same
// bars reach the core two ways.
//
// The spec uses Sma(3), which has a lookback: a condition over the raw close
// reads the same however many times a bar arrived, which is how this class of
// bug stays invisible.
#include <exception>
#include <iostream>
#include <string>
#include <vector>

#include "wickra_screener.hpp"

namespace {
const char *SPEC =
    R"({"universe":["AAA","BBB"],"condition":{"type":"cmp",)"
    R"("left":{"kind":"indicator","name":"Sma","params":[3]},)"
    R"("op":"gt","right":{"kind":"const","value":0.0}}})";

const char *DATASET =
    R"({"AAA":[)"
    R"({"time":1,"open":10,"high":10,"low":10,"close":10,"volume":1},)"
    R"({"time":2,"open":20,"high":20,"low":20,"close":20,"volume":1},)"
    R"({"time":3,"open":30,"high":30,"low":30,"close":30,"volume":1}],)"
    R"("BBB":[)"
    R"({"time":1,"open":40,"high":40,"low":40,"close":40,"volume":1},)"
    R"({"time":2,"open":50,"high":50,"low":50,"close":50,"volume":1},)"
    R"({"time":3,"open":60,"high":60,"low":60,"close":60,"volume":1}]})";

const std::vector<std::string> FEEDS = {
    R"({"cmd":"feed","symbol":"AAA","candle":{"time":1,"open":10,"high":10,"low":10,"close":10,"volume":1}})",
    R"({"cmd":"feed","symbol":"AAA","candle":{"time":2,"open":20,"high":20,"low":20,"close":20,"volume":1}})",
    R"({"cmd":"feed","symbol":"AAA","candle":{"time":3,"open":30,"high":30,"low":30,"close":30,"volume":1}})",
    R"({"cmd":"feed","symbol":"BBB","candle":{"time":1,"open":40,"high":40,"low":40,"close":40,"volume":1}})",
    R"({"cmd":"feed","symbol":"BBB","candle":{"time":2,"open":50,"high":50,"low":50,"close":50,"volume":1}})",
    R"({"cmd":"feed","symbol":"BBB","candle":{"time":3,"open":60,"high":60,"low":60,"close":60,"volume":1}})",
};
}  // namespace

int main() {
    try {
        wickra::Screener batchScreener(SPEC);
        const std::string batch =
            batchScreener.command(std::string(R"({"cmd":"scan","data":)") + DATASET + "}");

        wickra::Screener streaming(SPEC);
        for (const std::string &feed : FEEDS) {
            streaming.command(feed);
        }
        const std::string streamed = streaming.command(R"({"cmd":"evaluate"})");

        std::cout << "batch    : " << batch << "\n";
        std::cout << "streaming: " << streamed << "\n";

        if (batch != streamed) {
            std::cerr << "streaming != batch\n";
            return 1;
        }
        // Sma(3) over 10, 20, 30 is 20 and over 40, 50, 60 is 50. Fed twice each
        // they would be the means of 20, 30, 30 and 50, 60, 60 instead, so these
        // values are what tells the two apart.
        // A custom delimiter: the default R"( ... )" would end at the `)"` inside
        // `Sma(3)":`, which is a compile error rather than a wrong answer.
        if (batch.find(R"j("Sma(3)":20.0)j") == std::string::npos ||
            batch.find(R"j("Sma(3)":50.0)j") == std::string::npos) {
            std::cerr << "expected Sma(3) of 20.0 and 50.0 from three bars each\n";
            return 1;
        }
        std::cout << "streaming equals batch\n";
    } catch (const std::exception &e) {
        std::cerr << "failed: " << e.what() << "\n";
        return 1;
    }
    return 0;
}
