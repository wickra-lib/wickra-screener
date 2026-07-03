package wickra

import (
	"encoding/json"
	"strings"
	"testing"
)

const spec = `{"universe":["AAA","BBB"],"condition":{"type":"cmp","left":{"kind":"price","field":"close"},"op":"gt","right":{"kind":"const","value":10.0}}}`

func candle(close float64) map[string]float64 {
	return map[string]float64{
		"time": 1, "open": close, "high": close,
		"low": close, "close": close, "volume": 1,
	}
}

func TestVersion(t *testing.T) {
	if Version() == "" {
		t.Fatal("empty version")
	}
}

func TestScanReturnsMatchingSymbol(t *testing.T) {
	s, err := New(spec)
	if err != nil {
		t.Fatal(err)
	}
	defer s.Close()

	data := map[string][]map[string]float64{
		"AAA": {candle(5)},
		"BBB": {candle(15)},
	}
	cmd, err := json.Marshal(map[string]any{"cmd": "scan", "data": data})
	if err != nil {
		t.Fatal(err)
	}
	raw, err := s.Command(string(cmd))
	if err != nil {
		t.Fatal(err)
	}

	var report struct {
		Scanned int `json:"scanned"`
		Matches []struct {
			Symbol string `json:"symbol"`
		} `json:"matches"`
	}
	if err := json.Unmarshal([]byte(raw), &report); err != nil {
		t.Fatal(err)
	}
	if report.Scanned != 2 {
		t.Fatalf("expected scanned=2, got %d", report.Scanned)
	}
	if len(report.Matches) != 1 || report.Matches[0].Symbol != "BBB" {
		t.Fatalf("expected only BBB to match, got %+v", report.Matches)
	}
}

func TestInvalidSpec(t *testing.T) {
	if _, err := New("not json"); err == nil {
		t.Fatal("expected an error for an invalid spec")
	}
}

func TestUnknownCommandIsInBandError(t *testing.T) {
	s, err := New(spec)
	if err != nil {
		t.Fatal(err)
	}
	defer s.Close()

	// An unknown command is not a hard error: the C ABI returns a length and the
	// error surfaces in-band as {"ok":false,...} JSON.
	raw, err := s.Command(`{"cmd":"nope"}`)
	if err != nil {
		t.Fatalf("unexpected hard error: %v", err)
	}
	if !strings.Contains(raw, `"ok":false`) {
		t.Fatalf("expected an in-band error, got: %s", raw)
	}
}
