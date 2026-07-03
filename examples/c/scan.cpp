// A minimal C++ example: run a scan through the wickra-screener C ABI.
#include <cstddef>
#include <iostream>
#include <string>
#include <vector>

#include "wickra_screener.h"

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
    WickraScreener *screener = wickra_screener_new(SPEC);
    if (screener == nullptr) {
        std::cerr << "failed to build screener\n";
        return 1;
    }

    int len = wickra_screener_command(screener, CMD, nullptr, 0);
    if (len < 0) {
        std::cerr << "command failed: code " << len << "\n";
        wickra_screener_free(screener);
        return 1;
    }
    std::vector<char> buf(static_cast<std::size_t>(len) + 1);
    wickra_screener_command(screener, CMD, buf.data(),
                            static_cast<std::size_t>(buf.size()));

    std::cout << "wickra-screener " << wickra_screener_version() << "\n";
    std::cout << "scan: " << std::string(buf.data()) << "\n";

    wickra_screener_free(screener);
    return 0;
}
