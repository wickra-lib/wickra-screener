#!/usr/bin/env python3
"""Generate `golden/data-feeds.json` from the canonical candle universe.

The fed corpus is the same six symbols and the same candles as `data.json`, so a
fed report and a candle-only report are comparable, with one side feed per family
derived deterministically from each bar:

* ``reference``  the benchmark series (``sym-01``), for the pairwise family
* ``derivs``     a derivatives tick, for funding / open-interest indicators
* ``books``      a two-level order book around the bar, for book indicators
* ``trades``     one buy and one sell print inside the bar, for trade flow
* ``sections``   the market panel, for the breadth family

The feeds are functions of the candle rather than random, so this script is
reproducible and the committed output can be regenerated and diffed.

    python golden/generate_feeds.py
"""

from __future__ import annotations

import json
import pathlib

GOLDEN = pathlib.Path(__file__).resolve().parent
BENCHMARK = "sym-01"


def derivs_for(candle: dict, index: int) -> dict:
    """A derivatives tick priced off the bar."""
    close = candle["close"]
    return {
        "funding_rate": round(0.0001 + 0.00002 * ((index % 7) - 3), 8),
        "mark_price": round(close * 1.0002, 8),
        "index_price": round(close, 8),
        "futures_price": round(close * 1.0005, 8),
        "open_interest": round(1_000_000.0 + 5_000.0 * index, 8),
        "long_size": round(600_000.0 + 100.0 * index, 8),
        "short_size": round(400_000.0 + 50.0 * index, 8),
        "taker_buy_volume": round(candle["volume"] * 0.6, 8),
        "taker_sell_volume": round(candle["volume"] * 0.4, 8),
        "long_liquidation": round(10.0 + index % 5, 8),
        "short_liquidation": round(8.0 + index % 3, 8),
        "timestamp": candle["time"],
    }


def book_for(candle: dict) -> dict:
    """A two-level book straddling the close, best first and uncrossed."""
    close = candle["close"]
    tick = round(close * 0.0005, 8)
    return {
        "bids": [
            {"price": round(close - tick, 8), "size": 12.0},
            {"price": round(close - 2 * tick, 8), "size": 20.0},
        ],
        "asks": [
            {"price": round(close + tick, 8), "size": 9.0},
            {"price": round(close + 2 * tick, 8), "size": 18.0},
        ],
    }


def trades_for(candle: dict) -> list[dict]:
    """One aggressive buy and one aggressive sell inside the bar."""
    close = candle["close"]
    tick = round(close * 0.0005, 8)
    return [
        {
            "price": round(close + tick, 8),
            "size": round(candle["volume"] * 0.6, 8),
            "side": "buy",
            "timestamp": candle["time"],
        },
        {
            "price": round(close - tick, 8),
            "size": round(candle["volume"] * 0.4, 8),
            "side": "sell",
            "timestamp": candle["time"] + 1,
        },
    ]


def sections(data: dict[str, list[dict]]) -> list[dict]:
    """The market panel at each bar index, one member per symbol."""
    bars = min(len(candles) for candles in data.values())
    panel = []
    for index in range(bars):
        members = []
        for symbol in sorted(data):
            candles = data[symbol]
            candle = candles[index]
            previous = candles[index - 1]["close"] if index else candle["close"]
            window = candles[max(0, index - 20) : index]
            members.append(
                {
                    "change": round(candle["close"] - previous, 8),
                    "volume": round(candle["volume"], 8),
                    "new_high": bool(window) and candle["high"] > max(c["high"] for c in window),
                    "new_low": bool(window) and candle["low"] < min(c["low"] for c in window),
                }
            )
        panel.append({"members": members, "timestamp": data[BENCHMARK][index]["time"]})
    return panel


def main() -> None:
    data = json.loads((GOLDEN / "data.json").read_text(encoding="utf-8"))
    panel = sections(data)
    reference = data[BENCHMARK]

    fed = {}
    for symbol in sorted(data):
        candles = data[symbol]
        bars = len(candles)
        fed[symbol] = {
            "candles": candles,
            "reference": reference[:bars],
            "derivs": [derivs_for(c, i) for i, c in enumerate(candles)],
            "books": [book_for(c) for c in candles],
            "trades": [trades_for(c) for c in candles],
            "sections": panel[:bars],
        }

    out = GOLDEN / "data-feeds.json"
    out.write_text(json.dumps(fed, separators=(",", ":")) + "\n", encoding="utf-8", newline="\n")
    print(f"wrote {out} ({len(fed)} symbols, {len(next(iter(fed.values()))['candles'])} bars)")


if __name__ == "__main__":
    main()
