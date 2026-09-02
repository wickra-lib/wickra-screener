---
name: Bug report (detailed)
about: A bug that needs the full picture — a wrong value, a cross-language mismatch, a report that disagrees with itself
title: "[bug] "
labels: bug
---

Use this when the short bug form is not enough: a computed value looks wrong, one
language disagrees with another, or the behaviour depends on the data.

**What is wrong**
A clear description of the incorrect behaviour.

**The spec**

```json
# the smallest ScanSpec that shows it
```

**The universe**

```json
# the smallest dataset that shows it — a handful of candles, plus any side feeds
```

**The report you got**

```json
# the exact ScanReport JSON
```

**The report you expected**
Say what the right answer is and how you know: a hand computation, another tool,
or the same scan in another language.

**Does it reproduce in more than one language?**
- [ ] Rust · [ ] Python · [ ] Node.js · [ ] WASM · [ ] C · [ ] C++ · [ ] C# · [ ] Go · [ ] Java · [ ] R
- If only some: which agree and which do not?

**Does it reproduce in both modes?**
- [ ] Batch (`scan_batch`) · [ ] Streaming (`feed` + `evaluate`)
- The two are meant to produce identical reports; a difference between them is
  itself the bug.

**Environment**
- `wickra-screener` version / commit:
- Rust toolchain (`rustc -V`):
- OS and architecture:

**Anything you already ruled out**
Warmup, a missing side feed, symbols outside the universe, a stale bar.
