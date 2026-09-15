"""Cross-language golden: every binding must produce byte-identical scan JSON.

The fixtures live in the repository-root ``golden/`` directory (specs + shared
datasets + expected responses). The spec directory is globbed rather than listed,
so a spec added to the corpus is covered here without touching this file. A spec
named ``feeds_*`` scans ``data-feeds.json``, which carries the side feeds; every
other spec scans the candle-only ``data.json``. A missing corpus is a failure,
not a skip: a golden test that skips its own subject reports nothing.

Plain functions and plain asserts, so the module runs unchanged under pytest
(3.10 and up) and under ``run_without_pytest.py`` (the 3.9 row).
"""

import json
import pathlib

from wickra_screener import Screener

ROOT = pathlib.Path(__file__).resolve().parents[3]
GOLDEN = ROOT / "golden"


def _spec_files() -> list:
    specs = sorted((GOLDEN / "specs").glob("*.json"))
    assert specs, f"golden corpus not found under {GOLDEN}"
    return specs


def test_golden_scan_is_byte_identical() -> None:
    for spec_path in _spec_files():
        name = "data-feeds.json" if spec_path.stem.startswith("feeds_") else "data.json"
        dataset = json.loads((GOLDEN / name).read_text(encoding="utf-8"))
        expected = (GOLDEN / "expected" / f"{spec_path.stem}.json").read_text(
            encoding="utf-8"
        )
        screener = Screener(spec_path.read_text(encoding="utf-8"))
        response = screener.command(json.dumps({"cmd": "scan", "data": dataset}))
        assert response == expected.strip(), f"scan differs from the golden for {spec_path.stem}"
