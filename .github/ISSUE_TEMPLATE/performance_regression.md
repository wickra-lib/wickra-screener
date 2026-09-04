---
name: Performance regression
about: A scan got slower, or scales worse than it should
title: "[perf] "
labels: performance
---

**What got slower**
- Path: [ ] batch `scan_batch` · [ ] streaming `feed`/`evaluate` · [ ] a binding boundary
- Feature set: [ ] default (rayon `parallel`) · [ ] `--no-default-features` (sequential)

**Numbers**

| | Before | After |
|---|---|---|
| Version / commit | | |
| Universe size | | |
| Bars per symbol | | |
| Distinct indicators in the spec | | |
| Wall time | | |

**How you measured**
`cargo bench -p wickra-screener-bench`, a criterion comparison, or your own harness —
say which, and whether the machine was otherwise idle.

**The spec**

```json
# the ScanSpec you measured
```

**Does it scale differently, or is it a constant factor?**
Per-symbol throughput should stay roughly flat as the universe grows. A change
in the *shape* of the curve is a different problem from a change in its height.

**Environment**
- CPU (cores / model):
- OS and architecture:
- Rust toolchain (`rustc -V`):
