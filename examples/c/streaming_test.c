/* Streaming equals batch, through the C ABI's two-call idiom.
 *
 * wickra-screener-core proves this in Rust, but that says nothing about the boundary a
 * C caller crosses. Every reach behind this ABI asks for the response length
 * first and reads it second, and the golden corpus only ever sends
 * {"cmd":"scan"} -- a pure function of its payload. So the command ran twice on
 * every call and nothing noticed: each "feed" applied its candle a second time.
 *
 * The dataset here is written inline rather than read from golden/, because C
 * carries no JSON parser and splitting the corpus by hand would test the
 * splitter. What matters is that the same bars reach the core two ways.
 *
 * The spec uses Sma(3), which has a lookback: a condition over the raw close
 * reads the same however many times a bar arrived, which is precisely how this
 * class of bug stays invisible.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "wickra_screener.h"

static const char *SPEC =
    "{\"universe\":[\"AAA\",\"BBB\"],\"condition\":{\"type\":\"cmp\","
    "\"left\":{\"kind\":\"indicator\",\"name\":\"Sma\",\"params\":[3]},"
    "\"op\":\"gt\",\"right\":{\"kind\":\"const\",\"value\":0.0}}}";

/* Six bars: three per symbol, a rising ramp so Sma(3) is well defined. */
static const char *DATASET =
    "{\"AAA\":["
    "{\"time\":1,\"open\":10,\"high\":10,\"low\":10,\"close\":10,\"volume\":1},"
    "{\"time\":2,\"open\":20,\"high\":20,\"low\":20,\"close\":20,\"volume\":1},"
    "{\"time\":3,\"open\":30,\"high\":30,\"low\":30,\"close\":30,\"volume\":1}],"
    "\"BBB\":["
    "{\"time\":1,\"open\":40,\"high\":40,\"low\":40,\"close\":40,\"volume\":1},"
    "{\"time\":2,\"open\":50,\"high\":50,\"low\":50,\"close\":50,\"volume\":1},"
    "{\"time\":3,\"open\":60,\"high\":60,\"low\":60,\"close\":60,\"volume\":1}]}";

static const char *FEEDS[] = {
    "{\"cmd\":\"feed\",\"symbol\":\"AAA\",\"candle\":{\"time\":1,\"open\":10,\"high\":10,\"low\":10,\"close\":10,\"volume\":1}}",
    "{\"cmd\":\"feed\",\"symbol\":\"AAA\",\"candle\":{\"time\":2,\"open\":20,\"high\":20,\"low\":20,\"close\":20,\"volume\":1}}",
    "{\"cmd\":\"feed\",\"symbol\":\"AAA\",\"candle\":{\"time\":3,\"open\":30,\"high\":30,\"low\":30,\"close\":30,\"volume\":1}}",
    "{\"cmd\":\"feed\",\"symbol\":\"BBB\",\"candle\":{\"time\":1,\"open\":40,\"high\":40,\"low\":40,\"close\":40,\"volume\":1}}",
    "{\"cmd\":\"feed\",\"symbol\":\"BBB\",\"candle\":{\"time\":2,\"open\":50,\"high\":50,\"low\":50,\"close\":50,\"volume\":1}}",
    "{\"cmd\":\"feed\",\"symbol\":\"BBB\",\"candle\":{\"time\":3,\"open\":60,\"high\":60,\"low\":60,\"close\":60,\"volume\":1}}",
};
static const size_t FEED_COUNT = sizeof(FEEDS) / sizeof(FEEDS[0]);

/* Run one command through the documented two-call idiom. Caller frees. */
static char *run(WickraScreener *screener, const char *cmd) {
    int len = wickra_screener_command(screener, cmd, NULL, 0);
    if (len < 0) {
        fprintf(stderr, "command failed: code %d\n", len);
        return NULL;
    }
    char *buf = (char *)malloc((size_t)len + 1);
    if (!buf) {
        return NULL;
    }
    int written = wickra_screener_command(screener, cmd, buf, (size_t)len + 1);
    if (written != len) {
        fprintf(stderr, "second call returned %d, first said %d\n", written, len);
        free(buf);
        return NULL;
    }
    return buf;
}

int main(void) {
    char command[4096];
    int n = snprintf(command, sizeof(command), "{\"cmd\":\"scan\",\"data\":%s}", DATASET);
    if (n < 0 || (size_t)n >= sizeof(command)) {
        fprintf(stderr, "scan command did not fit\n");
        return 1;
    }

    WickraScreener *batch_screener = wickra_screener_new(SPEC);
    if (!batch_screener) {
        fprintf(stderr, "failed to build the batch screener\n");
        return 1;
    }
    char *batch = run(batch_screener, command);
    wickra_screener_free(batch_screener);
    if (!batch) {
        return 1;
    }

    WickraScreener *stream_screener = wickra_screener_new(SPEC);
    if (!stream_screener) {
        fprintf(stderr, "failed to build the streaming screener\n");
        free(batch);
        return 1;
    }
    for (size_t i = 0; i < FEED_COUNT; i++) {
        char *ack = run(stream_screener, FEEDS[i]);
        if (!ack) {
            wickra_screener_free(stream_screener);
            free(batch);
            return 1;
        }
        free(ack);
    }
    char *streamed = run(stream_screener, "{\"cmd\":\"evaluate\"}");
    wickra_screener_free(stream_screener);
    if (!streamed) {
        free(batch);
        return 1;
    }

    int equal = strcmp(batch, streamed) == 0;
    printf("batch    : %s\n", batch);
    printf("streaming: %s\n", streamed);

    /* Sma(3) over 10, 20, 30 is 20 and over 40, 50, 60 is 50. Fed twice each
     * they would be the means of 20, 30, 30 and 50, 60, 60 instead, so these
     * values are what tells the two apart. */
    int has_expected = strstr(batch, "\"Sma(3)\":20.0") != NULL &&
                       strstr(batch, "\"Sma(3)\":50.0") != NULL;

    free(batch);
    free(streamed);

    if (!equal) {
        fprintf(stderr, "streaming != batch\n");
        return 1;
    }
    if (!has_expected) {
        fprintf(stderr, "expected Sma(3) of 20.0 and 50.0 from three bars each\n");
        return 1;
    }
    printf("streaming equals batch\n");
    return 0;
}
