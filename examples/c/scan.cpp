// A minimal C++ example: run a scan through the wickra-screener C++ wrapper.
//
// The wrapper is header-only over the same C ABI the C example uses; what it
// removes is the handle bookkeeping and the two-call length protocol, which are
// the parts that are easy to get subtly wrong.
#include <exception>
#include <iostream>
#include <string>

#include "wickra_screener.hpp"

namespace {
const char *SPEC =
    R"({"universe":["AAA","BBB"],"condition":{"type":"cmp",)"
    R"("left":{"kind":"price","field":"close"},"op":"gt",)"
    R"("right":{"kind":"const","value":10.0}}})";

const char *CMD =
    R"({"cmd":"scan","data":{)"
    R"("AAA":[{"time":1,"open":5,"high":5,"low":5,"close":5,"volume":1}],)"
    R"("BBB":[{"time":1,"open":15,"high":15,"low":15,"close":15,"volume":1}]}})";
}  // namespace

int main() {
    try {
        wickra::Screener screener(SPEC);
        const std::string report = screener.command(CMD);

        std::cout << "wickra-screener " << wickra::Screener::version() << "\n";
        std::cout << "scan: " << report << "\n";

        // BBB closes at 15 and AAA at 5, so exactly one symbol matches.
        if (report.find("\"symbol\":\"BBB\"") == std::string::npos) {
            std::cerr << "expected BBB in the report\n";
            return 1;
        }
    } catch (const std::exception &e) {
        std::cerr << "failed: " << e.what() << "\n";
        return 1;
    }
    return 0;
}
