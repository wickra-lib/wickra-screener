# Documentation

These pages are the guides that live beside the code, because they describe how
this repository behaves and have to change in the same commit the behaviour does.

| Page | What it answers |
|------|-----------------|
| [CONDITIONS.md](CONDITIONS.md) | The shape of a `ScanSpec`: expressions, conditions, ranking, limits |
| [INDICATORS.md](INDICATORS.md) | Naming an indicator, its parameters, its warmup, and which side feed it needs |
| [CROSS_SECTION.md](CROSS_SECTION.md) | Rank, percentile and z-score across the universe; breadth; the market panel the screener assembles |
| [STREAMING.md](STREAMING.md) | Batch against streaming, and where the two are identical |
| [Cookbook.md](Cookbook.md) | Worked screens |
| [ARCHITECTURE.md](ARCHITECTURE.md) | The internals: how a scan is folded and evaluated |

The API reference for each language is generated from the source and published
on the site rather than committed here: <https://wickra.org>. Keeping it there is
deliberate — a second copy of the same reference in this repository would drift
from the code that generates it, and a reader opening `docs/` would have no way
to tell which copy was current.

What stays here is what a generator cannot produce: the meaning of a field, the
reason a case is refused rather than answered, and the worked examples.

Elsewhere in the repository:

- [`../ARCHITECTURE.md`](../ARCHITECTURE.md) — the crate and binding layout
- [`../BENCHMARKS.md`](../BENCHMARKS.md) — what is measured and how
- [`../golden/README.md`](../golden/README.md) — the cross-language corpus and how to regenerate it
- [`../CONTRIBUTING.md`](../CONTRIBUTING.md) — how to build, test and propose a change
- [`../THREAT_MODEL.md`](../THREAT_MODEL.md) — what the screener does and does not touch
