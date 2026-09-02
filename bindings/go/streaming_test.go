package wickra

// Streaming equals batch, driven through the JSON command boundary.
//
// screener-core proves this in Rust
// (crates/screener-core/tests/streaming_eq_batch.rs), but that says nothing about
// the boundary each language actually crosses. A binding reaches the core through
// Command, and the golden corpus only ever sends {"cmd":"scan"} -- so feed and
// evaluate were exercised in no language at all. A binding that mis-serialised a
// candle on the feed path, or dropped a field from the envelope, had no test to
// fail.
//
// Batch: one scan over the whole dataset.
// Streaming: feed each candle of each symbol in turn, then evaluate.
// Both return the core's compact JSON verbatim, so byte equality is the check.
//
// Only the candle-only specs are used. A feeds_* spec needs side feeds the
// streaming envelope carries per bar, and derived_breadth needs the market panel,
// which a streaming screener cannot derive: it sees one symbol's bar at a time
// and cannot know which other symbols print at that timestamp. That is a
// documented limitation (docs/CROSS_SECTION.md), not something to hide here.

import (
	"encoding/json"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"testing"
)

var streamingSpecs = []string{
	"momentum",
	"mean_reversion",
	"cross_section_rank",
	"breadth",
	"crossover",
	"compound",
}

// feedAll streams every candle of every symbol, in a stable symbol order.
func feedAll(t *testing.T, s *Screener, data map[string][]json.RawMessage) {
	t.Helper()
	symbols := make([]string, 0, len(data))
	for symbol := range data {
		symbols = append(symbols, symbol)
	}
	sort.Strings(symbols)
	for _, symbol := range symbols {
		for _, candle := range data[symbol] {
			envelope, err := json.Marshal(map[string]any{
				"cmd":    "feed",
				"symbol": symbol,
				"candle": candle,
			})
			if err != nil {
				t.Fatal(err)
			}
			if _, err := s.Command(string(envelope)); err != nil {
				t.Fatalf("feed %s: %v", symbol, err)
			}
		}
	}
}

func TestStreamingEqualsBatch(t *testing.T) {
	g := goldenDir()
	if g == "" {
		t.Skip("golden fixtures not present yet")
	}
	raw, err := os.ReadFile(filepath.Join(g, "data.json"))
	if err != nil {
		t.Fatal(err)
	}
	var data map[string][]json.RawMessage
	if err := json.Unmarshal(raw, &data); err != nil {
		t.Fatal(err)
	}

	compared := 0
	for _, name := range streamingSpecs {
		spec, err := os.ReadFile(filepath.Join(g, "specs", name+".json"))
		if err != nil {
			t.Fatalf("spec %s is missing from the corpus: %v", name, err)
		}

		batchScreener, err := New(string(spec))
		if err != nil {
			t.Fatal(err)
		}
		scan, err := json.Marshal(map[string]any{"cmd": "scan", "data": data})
		if err != nil {
			t.Fatal(err)
		}
		batch, err := batchScreener.Command(string(scan))
		batchScreener.Close()
		if err != nil {
			t.Fatal(err)
		}

		streamScreener, err := New(string(spec))
		if err != nil {
			t.Fatal(err)
		}
		feedAll(t, streamScreener, data)
		streamed, err := streamScreener.Command(`{"cmd":"evaluate"}`)
		streamScreener.Close()
		if err != nil {
			t.Fatal(err)
		}

		if strings.TrimSpace(streamed) != strings.TrimSpace(batch) {
			t.Fatalf("streaming != batch for spec %s\nstreaming: %s\nbatch:     %s",
				name, streamed, batch)
		}
		compared++
	}
	// A loop that compared nothing passes; say how many it actually did.
	if compared != len(streamingSpecs) {
		t.Fatalf("compared %d specs, expected %d", compared, len(streamingSpecs))
	}
}

func TestResetReturnsToThePreFeedState(t *testing.T) {
	g := goldenDir()
	if g == "" {
		t.Skip("golden fixtures not present yet")
	}
	raw, err := os.ReadFile(filepath.Join(g, "data.json"))
	if err != nil {
		t.Fatal(err)
	}
	var data map[string][]json.RawMessage
	if err := json.Unmarshal(raw, &data); err != nil {
		t.Fatal(err)
	}
	spec, err := os.ReadFile(filepath.Join(g, "specs", "momentum.json"))
	if err != nil {
		t.Fatal(err)
	}

	screener, err := New(string(spec))
	if err != nil {
		t.Fatal(err)
	}
	defer screener.Close()

	empty, err := screener.Command(`{"cmd":"evaluate"}`)
	if err != nil {
		t.Fatal(err)
	}

	feedAll(t, screener, data)
	fed, err := screener.Command(`{"cmd":"evaluate"}`)
	if err != nil {
		t.Fatal(err)
	}
	if fed == empty {
		t.Fatal("feeding the whole universe changed nothing")
	}

	if _, err := screener.Command(`{"cmd":"reset"}`); err != nil {
		t.Fatal(err)
	}
	back, err := screener.Command(`{"cmd":"evaluate"}`)
	if err != nil {
		t.Fatal(err)
	}
	if back != empty {
		t.Fatalf("reset did not return the screener to its pre-feed state\nwant: %s\ngot:  %s",
			empty, back)
	}
}
