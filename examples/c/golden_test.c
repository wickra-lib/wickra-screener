/* Cross-language golden parity, from C.
 *
 * Build the screener from each committed golden/specs/*.json, run a scan over
 * the matching committed dataset, and assert the response equals
 * golden/expected/<spec>.json byte-for-byte. The ABI returns the core's compact
 * command output verbatim, so byte equality is the exact cross-language parity
 * check -- the same one Python, Node, Go, C#, Java, R and WASM make.
 *
 * C has no directory API that is portable between POSIX and Windows, so the
 * spec list is globbed by CMake at configure time and written into
 * golden_specs.h. That keeps the property the other bindings get from a runtime
 * glob: a spec added to the corpus is covered here without editing this file.
 * A hand-maintained list would silently skip it, which is the failure this
 * whole corpus exists to prevent.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "wickra_screener.h"

#include "golden_specs.h" /* GOLDEN_DIR, GOLDEN_SPECS, GOLDEN_SPEC_COUNT */

/* Read a whole file. Caller frees. Returns NULL and reports on failure. */
static char *slurp(const char *path) {
    FILE *file = fopen(path, "rb");
    if (!file) {
        fprintf(stderr, "cannot open %s\n", path);
        return NULL;
    }
    if (fseek(file, 0, SEEK_END) != 0) {
        fclose(file);
        return NULL;
    }
    long size = ftell(file);
    if (size < 0 || fseek(file, 0, SEEK_SET) != 0) {
        fclose(file);
        return NULL;
    }
    char *buf = (char *)malloc((size_t)size + 1);
    if (!buf) {
        fclose(file);
        return NULL;
    }
    size_t got = fread(buf, 1, (size_t)size, file);
    fclose(file);
    buf[got] = '\0';
    return buf;
}

/* Trim ASCII whitespace in place and return the start of the trimmed text. */
static char *trim(char *text) {
    while (*text == ' ' || *text == '\n' || *text == '\r' || *text == '\t') {
        text++;
    }
    size_t len = strlen(text);
    while (len > 0) {
        char last = text[len - 1];
        if (last != ' ' && last != '\n' && last != '\r' && last != '\t') {
            break;
        }
        text[--len] = '\0';
    }
    return text;
}

static char *join(const char *a, const char *b, const char *c) {
    size_t len = strlen(a) + strlen(b) + strlen(c) + 1;
    char *out = (char *)malloc(len);
    if (out) {
        snprintf(out, len, "%s%s%s", a, b, c);
    }
    return out;
}

int main(void) {
    if (GOLDEN_SPEC_COUNT == 0) {
        fprintf(stderr, "no golden specs were configured; this would test nothing\n");
        return 1;
    }

    char *candles_only = slurp(GOLDEN_DIR "/data.json");
    char *fed = slurp(GOLDEN_DIR "/data-feeds.json");
    if (!candles_only || !fed) {
        free(candles_only);
        free(fed);
        return 1;
    }
    char *candles_only_trimmed = trim(candles_only);
    char *fed_trimmed = trim(fed);

    int failures = 0;
    for (size_t i = 0; i < GOLDEN_SPEC_COUNT; i++) {
        const char *name = GOLDEN_SPECS[i];

        char *spec_path = join(GOLDEN_DIR "/specs/", name, "");
        char *expected_path = join(GOLDEN_DIR "/expected/", name, "");
        char *spec = spec_path ? slurp(spec_path) : NULL;
        char *expected_raw = expected_path ? slurp(expected_path) : NULL;
        free(spec_path);
        free(expected_path);
        if (!spec || !expected_raw) {
            fprintf(stderr, "%s: missing spec or expected file\n", name);
            free(spec);
            free(expected_raw);
            failures++;
            continue;
        }
        char *expected = trim(expected_raw);

        /* A spec named feeds_* scans the dataset carrying the side feeds. */
        const char *dataset = strncmp(name, "feeds_", 6) == 0 ? fed_trimmed : candles_only_trimmed;

        char *command = join("{\"cmd\":\"scan\",\"data\":", dataset, "}");
        WickraScreener *screener = command ? wickra_screener_new(spec) : NULL;
        if (!screener) {
            fprintf(stderr, "%s: spec rejected\n", name);
            free(command);
            free(spec);
            free(expected_raw);
            failures++;
            continue;
        }

        int len = wickra_screener_command(screener, command, NULL, 0);
        char *got_raw = len >= 0 ? (char *)malloc((size_t)len + 1) : NULL;
        if (got_raw) {
            wickra_screener_command(screener, command, got_raw, (size_t)len + 1);
            char *got = trim(got_raw);
            if (strcmp(got, expected) != 0) {
                fprintf(stderr, "%s: mismatch\n  expected: %s\n  got:      %s\n",
                        name, expected, got);
                failures++;
            }
        } else {
            fprintf(stderr, "%s: command failed with code %d\n", name, len);
            failures++;
        }

        free(got_raw);
        wickra_screener_free(screener);
        free(command);
        free(spec);
        free(expected_raw);
    }

    free(candles_only);
    free(fed);

    if (failures > 0) {
        fprintf(stderr, "%d of %zu golden specs did not match\n", failures, GOLDEN_SPEC_COUNT);
        return 1;
    }
    printf("all %zu golden specs are byte-identical from C\n", GOLDEN_SPEC_COUNT);
    return 0;
}
