package org.wickra.screener;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assumptions.assumeTrue;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

// Streaming equals batch, driven through the JSON command boundary.
//
// screener-core proves this in Rust
// (crates/screener-core/tests/streaming_eq_batch.rs), but that says nothing about
// the boundary each language actually crosses. A binding reaches the core through
// command(), and the golden corpus only ever sends {"cmd":"scan"} -- so feed and
// evaluate were exercised in no language at all. That is how the C ABI shipped a
// command that ran twice under the two-call idiom this binding uses: scan is a
// pure function of its payload, so it looked correct while every feed applied its
// candle twice.
//
// Batch: one scan over the whole dataset.
// Streaming: feed each candle of each symbol in turn, then evaluate.
// Both return the core's compact JSON verbatim, so string equality is the check.
//
// Only the candle-only specs are used. A feeds_* spec needs side feeds the
// streaming envelope carries per bar, and derived_breadth needs the market panel,
// which a streaming screener cannot derive: it sees one symbol's bar at a time and
// cannot know which other symbols print at that timestamp. That is a documented
// limitation (docs/CROSS_SECTION.md), not something to hide here.
class StreamingTest {
    private static final String[] SPECS = {
        "momentum", "mean_reversion", "cross_section_rank", "breadth", "crossover", "compound",
    };

    private static Path findGolden() {
        Path dir = Path.of("").toAbsolutePath();
        for (int i = 0; i < 8 && dir != null; i++) {
            Path g = dir.resolve("golden");
            if (Files.isDirectory(g.resolve("specs"))) {
                return g;
            }
            dir = dir.getParent();
        }
        return null;
    }

    // The binding carries no JSON library -- it is a thin FFI wrapper, and adding
    // a dependency to a test is a dependency all the same. The fixture is
    // machine-generated and its shape is fixed ({"symbol":[{candle}, ...], ...}),
    // so a depth counter that respects strings and escapes is enough. It is
    // guarded by the assertions in the tests: a splitter that returned nothing
    // would fail there rather than pass quietly.
    private static Map<String, List<String>> splitDataset(String json) {
        Map<String, List<String>> out = new LinkedHashMap<>();
        int i = json.indexOf('{');
        while (i >= 0 && i < json.length()) {
            int keyStart = json.indexOf('"', i);
            if (keyStart < 0) {
                break;
            }
            int keyEnd = json.indexOf('"', keyStart + 1);
            String symbol = json.substring(keyStart + 1, keyEnd);
            int arrayStart = json.indexOf('[', keyEnd);
            if (arrayStart < 0) {
                break;
            }
            int arrayEnd = matchingClose(json, arrayStart, '[', ']');
            out.put(symbol, splitObjects(json.substring(arrayStart + 1, arrayEnd)));
            i = arrayEnd + 1;
            int next = json.indexOf(',', i);
            if (next < 0) {
                break;
            }
            i = next;
        }
        return out;
    }

    private static List<String> splitObjects(String arrayBody) {
        List<String> objects = new ArrayList<>();
        int i = 0;
        while (i < arrayBody.length()) {
            int start = arrayBody.indexOf('{', i);
            if (start < 0) {
                break;
            }
            int end = matchingClose(arrayBody, start, '{', '}');
            objects.add(arrayBody.substring(start, end + 1));
            i = end + 1;
        }
        return objects;
    }

    /** Index of the bracket closing the one at {@code open}, respecting strings. */
    private static int matchingClose(String text, int open, char opener, char closer) {
        int depth = 0;
        boolean inString = false;
        for (int i = open; i < text.length(); i++) {
            char c = text.charAt(i);
            if (inString) {
                if (c == '\\') {
                    i++;
                } else if (c == '"') {
                    inString = false;
                }
                continue;
            }
            if (c == '"') {
                inString = true;
            } else if (c == opener) {
                depth++;
            } else if (c == closer) {
                depth--;
                if (depth == 0) {
                    return i;
                }
            }
        }
        throw new IllegalStateException("unbalanced JSON in the golden fixture");
    }

    private static void feedAll(Screener screener, Map<String, List<String>> data) {
        for (Map.Entry<String, List<String>> entry : data.entrySet()) {
            for (String candle : entry.getValue()) {
                screener.command(
                        "{\"cmd\":\"feed\",\"symbol\":\""
                                + entry.getKey()
                                + "\",\"candle\":"
                                + candle
                                + "}");
            }
        }
    }

    @Test
    void streamingEqualsBatch() throws IOException {
        Path golden = findGolden();
        assumeTrue(golden != null, "golden fixtures not present yet");

        String dataText = Files.readString(golden.resolve("data.json")).strip();
        Map<String, List<String>> data = splitDataset(dataText);
        // A splitter that produced nothing would make every comparison trivially
        // true, so say what it found.
        assertTrue(data.size() >= 2, "expected several symbols, got " + data.size());
        for (Map.Entry<String, List<String>> entry : data.entrySet()) {
            assertTrue(
                    entry.getValue().size() >= 10,
                    entry.getKey() + " yielded only " + entry.getValue().size() + " candles");
        }

        int compared = 0;
        for (String name : SPECS) {
            Path specPath = golden.resolve("specs").resolve(name + ".json");
            assertTrue(Files.exists(specPath), "spec " + name + " is missing from the corpus");
            String spec = Files.readString(specPath);

            String batch;
            try (Screener screener = new Screener(spec)) {
                batch = screener.command("{\"cmd\":\"scan\",\"data\":" + dataText + "}").strip();
            }

            String streamed;
            try (Screener screener = new Screener(spec)) {
                feedAll(screener, data);
                streamed = screener.command("{\"cmd\":\"evaluate\"}").strip();
            }

            assertEquals(batch, streamed, "streaming != batch for spec " + name);
            compared++;
        }
        assertEquals(SPECS.length, compared, "not every spec was compared");
    }

    @Test
    void resetReturnsToThePreFeedState() throws IOException {
        Path golden = findGolden();
        assumeTrue(golden != null, "golden fixtures not present yet");

        Map<String, List<String>> data =
                splitDataset(Files.readString(golden.resolve("data.json")).strip());
        String spec = Files.readString(golden.resolve("specs").resolve("momentum.json"));

        try (Screener screener = new Screener(spec)) {
            String empty = screener.command("{\"cmd\":\"evaluate\"}");

            feedAll(screener, data);
            assertNotEquals(
                    empty,
                    screener.command("{\"cmd\":\"evaluate\"}"),
                    "feeding the whole universe changed nothing");

            screener.command("{\"cmd\":\"reset\"}");
            assertEquals(
                    empty,
                    screener.command("{\"cmd\":\"evaluate\"}"),
                    "reset did not return the screener to its pre-feed state");
        }
    }
}
