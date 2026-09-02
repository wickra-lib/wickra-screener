---
name: Feature request (detailed)
about: A new condition kind, expression form, feed or binding surface
title: "[feat] "
labels: enhancement
---

**The screen you cannot write today**
Describe the screen in words first, then show how far the current spec gets you.

```json
# the closest ScanSpec you can write now, and where it falls short
```

**What you would want to write**

```json
# the spec you wish worked
```

**Why it belongs in the core rather than around it**
A screen is data, not code, so anything added here has to cross the C ABI and
WASM unchanged and mean the same thing in ten languages. Say why this cannot be
a step the caller takes before or after the scan.

**Does the strategy spec already have it?**
`wickra-backtest` has an operand and condition grammar the screener mirrors. If
this exists there, name it — matching it is cheaper and keeps the two languages
the same.

**Effect on existing specs**
- [ ] Additive: existing specs and golden reports are untouched
- [ ] Changes an existing meaning (say which, and why that is right)

**Alternatives you considered**
