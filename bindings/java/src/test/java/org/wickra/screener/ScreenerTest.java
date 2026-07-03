package org.wickra.screener;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

class ScreenerTest {
    private static final String SPEC =
            "{\"universe\":[\"AAA\",\"BBB\"],\"condition\":{\"type\":\"cmp\","
                    + "\"left\":{\"kind\":\"price\",\"field\":\"close\"},\"op\":\"gt\","
                    + "\"right\":{\"kind\":\"const\",\"value\":10.0}}}";

    private static String candle(int close) {
        return "{\"time\":1,\"open\":" + close + ",\"high\":" + close
                + ",\"low\":" + close + ",\"close\":" + close + ",\"volume\":1}";
    }

    @Test
    void versionIsNonEmpty() {
        assertFalse(Screener.version().isEmpty());
    }

    @Test
    void scanReturnsMatchingSymbol() {
        try (Screener screener = new Screener(SPEC)) {
            String cmd = "{\"cmd\":\"scan\",\"data\":{"
                    + "\"AAA\":[" + candle(5) + "],"
                    + "\"BBB\":[" + candle(15) + "]}}";
            String raw = screener.command(cmd);
            assertTrue(raw.contains("\"scanned\":2"), raw);
            assertTrue(raw.contains("\"symbol\":\"BBB\""), raw);
            assertFalse(raw.contains("\"symbol\":\"AAA\""), raw);
        }
    }

    @Test
    void invalidSpecThrows() {
        assertThrows(IllegalArgumentException.class, () -> new Screener("not json"));
    }

    @Test
    void unknownCommandIsInBandError() {
        try (Screener screener = new Screener(SPEC)) {
            // An unknown command is not a hard error: the ABI returns a length and
            // the error surfaces in-band as {"ok":false,...} JSON.
            String raw = screener.command("{\"cmd\":\"nope\"}");
            assertEquals(true, raw.contains("\"ok\":false"), raw);
        }
    }
}
