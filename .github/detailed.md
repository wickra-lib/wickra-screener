# Pull request (detailed)

The default template is enough for most changes. Use this one — append
`?template=detailed.md` to the compare URL, or copy the sections below into the
description — when the change is large enough that a reviewer needs the reasoning
and not only the diff: a change to what a report says, a new condition or feed, a
performance claim, or anything that touches more than one language.

## What was wrong

State the behaviour as it is today, with the evidence. A quoted line of code, a
command and its output, or a report that disagrees with itself is worth more than
a description. If nothing was wrong and this is new capability, say what could not
be expressed before.

## What this does

The change, in the order a reader needs it rather than the order you wrote it.

## Decisions a reviewer should not have to reconstruct

Anything you chose between. The alternatives you rejected and why — especially
where the honest answer was a refusal rather than a plausible-looking number.

## Effect on the report

- [ ] No field, value, ordering or key changes
- [ ] Additive: a new field, omitted when empty, so existing output is unchanged
- [ ] Changes existing output — say which specs move and why that is right

Every binding returns the core's serialization verbatim, so a report change is a
change in all ten languages at once.

## Golden corpus

- [ ] Untouched
- [ ] Re-blessed — paste the diff, and say why each moved value is correct.
      Blessing a diff you have not read is how a wrong number becomes the
      expected one.

## Cross-language

- [ ] Rust only (core internals, no surface change)
- [ ] The surface changed — list the bindings you ran locally and the ones you
      are leaving to CI

## Verification

The commands you actually ran, with their results:

```
cargo fmt --all
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo clippy --workspace --all-targets --no-default-features -- -D warnings
cargo check --manifest-path fuzz/Cargo.toml   # detached workspace: not built by --workspace
cargo deny check
```

## Checklist

- [ ] `CHANGELOG.md` updated under `[Unreleased]`
- [ ] Conditions stay data (a serde `ScanSpec`), never Rust closures
- [ ] No indicator reimplemented here — the registry owns them
- [ ] An unsupported case is a refusal that names what is missing, not a silent
      zero or an empty result
- [ ] Documentation changed alongside the behaviour it describes
