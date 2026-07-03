/* A minimal C example: run a scan through the wickra-screener C ABI. */
#include <stdio.h>
#include <stdlib.h>

#include "wickra_screener.h"

static const char *SPEC =
    "{\"universe\":[\"AAA\",\"BBB\"],\"condition\":{\"type\":\"cmp\","
    "\"left\":{\"kind\":\"price\",\"field\":\"close\"},\"op\":\"gt\","
    "\"right\":{\"kind\":\"const\",\"value\":10.0}}}";

static const char *CMD =
    "{\"cmd\":\"scan\",\"data\":{"
    "\"AAA\":[{\"time\":1,\"open\":5,\"high\":5,\"low\":5,\"close\":5,\"volume\":1}],"
    "\"BBB\":[{\"time\":1,\"open\":15,\"high\":15,\"low\":15,\"close\":15,\"volume\":1}]}}";

int main(void) {
    WickraScreener *screener = wickra_screener_new(SPEC);
    if (!screener) {
        fprintf(stderr, "failed to build screener\n");
        return 1;
    }

    /* Length-out protocol: learn the length, then read into a caller buffer. */
    int len = wickra_screener_command(screener, CMD, NULL, 0);
    if (len < 0) {
        fprintf(stderr, "command failed: code %d\n", len);
        wickra_screener_free(screener);
        return 1;
    }
    char *buf = (char *)malloc((size_t)len + 1);
    if (!buf) {
        wickra_screener_free(screener);
        return 1;
    }
    wickra_screener_command(screener, CMD, buf, (size_t)len + 1);

    printf("wickra-screener %s\n", wickra_screener_version());
    printf("scan: %s\n", buf);

    free(buf);
    wickra_screener_free(screener);
    return 0;
}
