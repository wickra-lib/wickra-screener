# Threat Model

`wickra-screener` is analysis software. It evaluates conditions over market data
and places no orders, opens no authenticated connections on its default path, and
holds no secret key material. The attack surface is correspondingly narrow: it is
dominated by the parsing of **untrusted input** — a `ScanSpec` and a symbol
universe supplied by the caller — as it crosses the C ABI and WASM boundary.

## Assets

- The **`ScanSpec` and universe data** a caller supplies. These are inputs, not
  secrets, but a malformed or hostile one must never crash or corrupt the host.
- The **integrity and determinism** of the `ScanReport`: the same spec and data
  must always produce the same result, in every language.
- The **host process** embedding a binding. A scan must not be able to take it
  down (panic across FFI, unbounded allocation) or read memory it should not.

There is intentionally **no secret asset** on the default path — no API keys, no
credentials, no order flow.

## Trust boundaries

- **Caller → core.** Everything arriving through `Screener::command` (spec, data,
  command) is untrusted and validated (`ScanSpec::validate`) before use.
- **Binding → C ABI hub.** The hub is the one place `unsafe` is allowed. It wraps
  every call in `catch_unwind`, guards null pointers, and uses a length-out
  buffer protocol so no panic or invalid pointer crosses into C / Go / C# / Java
  / R.
- **Optional `live` feature.** Only this pulls `wickra-exchange` to read a public
  symbol universe; it adds a network read but still no credentials or orders.

## Guarantees the code is held to

- `unsafe_code = "forbid"` workspace-wide; only `bindings/c` re-allows it locally.
- No panic crosses the FFI boundary; errors are returned as JSON, never as an
  abort.
- Parsing is bounded and total — a hostile spec or dataset yields an error, not
  an unbounded allocation or a hang.
- The parallel (rayon) and sequential (WASM) scan paths produce a byte-identical
  report, so parallelism introduces no nondeterminism.

## Out of scope

- Incorrect indicator mathematics — a functional bug, handled through normal
  issues and tests, not a vulnerability.
- Vulnerabilities in third-party crates, which are tracked and triaged through
  `deny.toml` and `osv-scanner.toml`.
- Resource exhaustion a caller inflicts on **their own** process by deliberately
  feeding an enormous universe; the core bounds its own allocations but cannot
  bound the caller's data volume.
