"""A runnable Python example: scan a small universe through the binding.

    pip install wickra-screener
    python examples/python/scan.py
"""

import json

from wickra_screener import Screener

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


def candle(close: float) -> dict:
    return {
        "time": 1,
        "open": close,
        "high": close,
        "low": close,
        "close": close,
        "volume": 1.0,
    }


def main() -> None:
    screener = Screener(SPEC)
    response = screener.command(
        json.dumps(
            {
                "cmd": "scan",
                "data": {"AAA": [candle(5.0)], "BBB": [candle(15.0)]},
            }
        )
    )
    report = json.loads(response)

    print(f"wickra-screener {Screener.version()}")
    print(response)
    for match in report["matches"]:
        print(f"  matched: {match['symbol']}")


if __name__ == "__main__":
    main()
