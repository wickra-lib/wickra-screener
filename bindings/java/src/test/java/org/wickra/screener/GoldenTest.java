package org.wickra.screener;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assumptions.assumeTrue;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.stream.Stream;
import org.junit.jupiter.api.Test;

// Cross-language golden parity: build the screener from each committed
// golden/specs/*.json, run a scan over the shared golden/data.json, and assert
// the response equals golden/expected/<spec>.json byte-for-byte. The binding
// returns the core's compact command_json string verbatim, so byte equality is
// the exact cross-language parity check. The fixtures arrive in a later phase;
// until then the test is skipped.
class GoldenTest {
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

    @Test
    void goldenScansAreByteIdentical() throws IOException {
        Path golden = findGolden();
        assumeTrue(golden != null, "golden fixtures not present yet");

        String dataset = Files.readString(golden.resolve("data.json")).strip();
        String cmdPrefix = "{\"cmd\":\"scan\",\"data\":";
        try (Stream<Path> specs = Files.list(golden.resolve("specs"))) {
            for (Path specPath : specs.filter(p -> p.toString().endsWith(".json")).toList()) {
                String spec = Files.readString(specPath);
                String name = specPath.getFileName().toString();
                String expected = Files.readString(golden.resolve("expected").resolve(name)).strip();
                try (Screener screener = new Screener(spec)) {
                    String raw = screener.command(cmdPrefix + dataset + "}");
                    assertEquals(expected, raw.strip(), name);
                }
            }
        }
    }
}
