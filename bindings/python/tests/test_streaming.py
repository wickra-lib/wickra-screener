"""Streaming equals batch, driven through the JSON command boundary.

``wickra-screener-core`` proves this in Rust
(``crates/wickra-screener-core/tests/streaming_eq_batch.rs``), but that says nothing
about the boundary each language actually crosses. A binding reaches the core
through ``command``, and the golden corpus only ever sends ``{"cmd":"scan"}`` --
so ``feed`` and ``evaluate`` were exercised in no language at all. A binding that
mis-serialised a candle on the feed path, or dropped a field from the envelope,
had no test to fail.

Batch: one ``scan`` over the whole dataset.
Streaming: ``feed`` each candle of each symbol in turn, then ``evaluate``.
Both return the core's compact JSON verbatim, so byte equality is the check.

Only the candle-only specs are used. A ``feeds_*`` spec needs side feeds the
streaming envelope carries per bar, and ``derived_breadth`` needs the market
panel, which a streaming screener cannot derive: it sees one symbol's bar at a
time and cannot know which other symbols print at that timestamp. That is a
documented limitation (``docs/CROSS_SECTION.md``), not something to hide here.
"""

import json
import pathlib

import pytest

from wickra_screener import Screener

ROOT = pathlib.Path(__file__).resolve().parents[3]
GOLDEN = ROOT / "golden"

SPECS = [
    "momentum",
    "mean_reversion",
    "cross_section_rank",
    "breadth",
    "crossover",
    "compound",
]


def _dataset() -> dict:
    return json.loads((GOLDEN / "data.json").read_text(encoding="utf-8"))


def _feed_all(screener: Screener, data: dict) -> None:
    for symbol in sorted(data):
        for candle in data[symbol]:
            screener.command(
                json.dumps({"cmd": "feed", "symbol": symbol, "candle": candle})
            )


@pytest.mark.skipif(not (GOLDEN / "specs").exists(), reason="golden fixtures absent")
@pytest.mark.parametrize("name", SPECS)
def test_streaming_equals_batch(name: str) -> None:
    spec_path = GOLDEN / "specs" / f"{name}.json"
    assert spec_path.exists(), f"spec {name} is missing from the corpus"
    spec = spec_path.read_text(encoding="utf-8")
    data = _dataset()

    batch = Screener(spec).command(json.dumps({"cmd": "scan", "data": data})).strip()

    streaming = Screener(spec)
    _feed_all(streaming, data)
    streamed = streaming.command(json.dumps({"cmd": "evaluate"})).strip()

    assert streamed == batch, f"streaming != batch for spec {name}"


@pytest.mark.skipif(not (GOLDEN / "specs").exists(), reason="golden fixtures absent")
def test_reset_returns_to_the_pre_feed_state() -> None:
    spec = (GOLDEN / "specs" / "momentum.json").read_text(encoding="utf-8")
    data = _dataset()

    screener = Screener(spec)
    empty = screener.command(json.dumps({"cmd": "evaluate"}))

    _feed_all(screener, data)
    assert screener.command(json.dumps({"cmd": "evaluate"})) != empty, (
        "feeding the whole universe changed nothing"
    )

    screener.command(json.dumps({"cmd": "reset"}))
    assert screener.command(json.dumps({"cmd": "evaluate"})) == empty, (
        "reset did not return the screener to its pre-feed state"
    )
