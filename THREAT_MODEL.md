# Threat Model

`wickra-screener` is analysis software. It evaluates conditions over market data
and places no orders, opens no authenticated connections on its default path, and
holds no secret key material. The attack surface is correspondingly narrow: it is
dominated by the parsing of **untrusted input** — a `ScanSpec` and a symbol
universe supplied by the caller — as it crosses the C ABI and WASM boundary.

This document complements the security assurance case in
[`SECURITY.md`](SECURITY.md).

## Assets

- The **`ScanSpec` and universe data** a caller supplies. These are inputs, not
  secrets, but a malformed or hostile one must never crash or corrupt the host.
- The **integrity and determinism** of the `ScanReport`: the same spec and data
  must always produce the same result, in every language.
- The **host process** embedding a binding. A scan must not be able to take it
  down (panic across FFI, unbounded allocation) or read memory it should not.
- The **integrity of published artifacts** — the crates, wheels, npm tarballs,
  NuGet package, jar and C ABI archives users install.
- The **build and release pipeline** and its secrets (publishing tokens).

There is intentionally **no secret asset** on the default path — no API keys, no
credentials, no order flow.

## Actors / trust boundaries

- **Library consumer** (trusted) — calls the API. The *data* they pass may come
  from an untrusted source, so a spec, a dataset and a command envelope are
  treated as untrusted even though the caller is not.
- **Caller → core.** Everything arriving through `Screener::command` (spec, data,
  command) is untrusted and validated (`ScanSpec::validate`) before use.
- **Binding → C ABI hub.** The hub is the one place `unsafe` is allowed. It wraps
  every call in `catch_unwind`, guards null pointers, and uses a length-out
  buffer protocol so no panic or invalid pointer crosses into C / C++ / Go / C# /
  Java / R.
- **Optional `live` feature.** Only this pulls `wickra-exchange` to read a public
  symbol universe; data crosses a network boundary from an exchange over TLS. It
  adds a network read but still no credentials and no orders.
- **Contributors** (semi-trusted) — propose changes via pull requests.
- **Supply chain** — upstream dependencies and the CI/CD platform.

## Threats and mitigations

| Threat | Mitigation |
| --- | --- |
| Memory-safety exploit (buffer overflow, use-after-free) via crafted input | Safe Rust throughout; the workspace sets `unsafe_code = "forbid"`, so the compiler precludes these classes outside the one boundary below. |
| Misuse of the C ABI FFI boundary (invalid handle, undersized buffer) | `bindings/c` is the sole `unsafe` surface. Its shim adds no scan logic, NULL-checks every handle and string, validates UTF-8, writes only into caller-sized buffers, and catches panics rather than letting one unwind across the boundary. A caller passing a non-NULL but dangling pointer is undefined behaviour by C's own contract — out of scope, as for any C library. |
| A mutating command applied more than once by the two-call protocol | A response produced but not delivered is held on the handle and returned by the call that reads it, so a command runs once per delivered response. Pinned by three C ABI unit tests and by a streaming-equals-batch test in every binding. |
| Denial of service via malformed or degenerate input (NaN, infinities, extreme magnitudes, a hostile spec) | A spec is validated before a scan begins, including rejecting one whose indicators need a feed the caller cannot supply; a non-finite division yields no value; parsing is bounded and total. The command envelope, spec parser and feed conversions are exercised by coverage-guided fuzzing in CI. |
| Silently incorrect or nondeterministic results | The golden corpus holds every binding to byte-identical output; the parallel (rayon) and sequential (WASM) paths must agree byte-for-byte; streaming must equal batch in all nine reaches; coverage on the core is tracked by Codecov. |
| Integer overflow / panics | `clippy::pedantic` with `-D warnings` under both feature sets; overflow checks enabled in test and fuzz builds. |
| Adversary-in-the-middle on the optional live feed | The connection uses TLS via the platform library; transport security is delegated to that reviewed implementation. No credential is sent and no order is placed. |
| Compromised dependency (supply chain) | Dependencies pinned (`Cargo.lock`, `package-lock.json`, hash-locked CI requirements), monitored by Dependabot, audited by `cargo-deny` and `osv-scanner` on every change. Every GitHub Action is pinned by commit SHA. |
| Malicious or accidental change to `main` | All changes flow through pull requests with required CI; commits and tags are signed; static analysis (CodeQL across every binding language, Clippy) and fuzzing run on every change. Branch protection is not yet configured on this repository — until it is, the pull-request flow and signing are conventions the maintainer keeps rather than rules the platform enforces. |
| Compromised CI / leaked secrets | Workflows use least-privilege `permissions:`; secrets live only as encrypted GitHub Actions secrets at the organisation level, with NuGet publishing over OIDC and no stored token; secret scanning with push protection is enabled; workflows are linted by `zizmor` and `actionlint`. |
| Tampered release artifact | Releases are built in CI, tags are signed, a guard job refuses to publish from anything but a `v*` tag, and assets carry build provenance attestations (verifiable with `gh attestation verify`). |

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
- The screener implements no authentication, authorization or cryptography of its
  own, stores no user data, and exposes no network listener; those threat classes
  do not apply.
- Vulnerabilities in third-party crates, which are tracked and triaged as
  exploitability (VEX) records through `deny.toml` and `osv-scanner.toml` (see
  [`SECURITY.md`](SECURITY.md)).
- Resource exhaustion a caller inflicts on **their own** process by deliberately
  feeding an enormous universe; the core bounds its own allocations but cannot
  bound the caller's data volume.

## Maintenance

This threat model is reviewed when the architecture changes materially — for
example a new input family, a new network feature, a new binding, or a new
release channel.
