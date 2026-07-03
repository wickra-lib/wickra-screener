"""Smoke test: construct a screener, run a scan, parse the report."""

import json

from wickra_screener import Screener, __version__

SPEC = json.dumps(
    {
        "universe": ["AAA", "BBB"],
        "condition": {
            "type": "cmp",
            "left": {"kind": "price", "field": "close"},
            "op": "gt",
            "right": {"kind": "const", "value": 10.0},
        },
    }
)


def _candle(close: float) -> dict:
    return {
        "time": 1,
        "open": close,
        "high": close,
        "low": close,
        "close": close,
        "volume": 1.0,
    }


def test_scan_roundtrip() -> None:
    screener = Screener(SPEC)
    response = screener.command(
        json.dumps(
            {
                "cmd": "scan",
                "data": {"AAA": [_candle(5.0)], "BBB": [_candle(15.0)]},
            }
        )
    )
    report = json.loads(response)
    assert report["scanned"] == 2
    assert [m["symbol"] for m in report["matches"]] == ["BBB"]


def test_version_matches_module() -> None:
    assert Screener.version() == __version__


def test_bad_spec_raises() -> None:
    try:
        Screener("not json")
    except ValueError:
        return
    raise AssertionError("expected ValueError for a malformed spec")
