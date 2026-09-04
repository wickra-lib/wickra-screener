# Security Policy

`wickra-screener` is analysis software: it evaluates data-driven conditions over
market data and places no orders. It holds no order-secret material and opens no
authenticated connections on its default path (the optional `live` universe
feature only reads public data). The attack surface is therefore narrow —
principally the parsing of untrusted `ScanSpec` / universe data as it crosses the
C ABI and WASM boundary. See [THREAT_MODEL.md](THREAT_MODEL.md) for the asset
inventory and trust boundaries.

## Supported versions

Security fixes are applied to the latest released version, `0.1.0`, only; please
upgrade to the newest release before reporting an issue.

| Version | Supported |
|---------|-----------|
| 0.1.0 (latest) | ✅ |

## Reporting a vulnerability

**Please do not open a public issue, pull request or discussion for security
problems.** Report privately through either channel:

- GitHub's [private vulnerability reporting](https://github.com/wickra-lib/wickra-screener/security/advisories/new)
  ("Report a vulnerability" under the repository's **Security** tab), or
- email **support@wickra.org** with a subject line starting with
  `[wickra-screener security]`.

Please include:

- the affected version or commit, and the platform / language binding,
- a description of the issue and its impact,
- steps to reproduce, ideally a minimal proof of concept.

## What to expect

- An acknowledgement within **5 working days**.
- An assessment and, if confirmed, a planned fix with a target release.
- Coordinated disclosure: we will agree on a disclosure date with you and credit
  you in the release notes unless you prefer to stay anonymous.

## Scope

In scope: memory-safety or panic-across-FFI flaws in the C ABI hub and its
buffer protocol, denial-of-service through a hostile `ScanSpec` or dataset (for
example unbounded allocation while parsing), any input that makes a binding
return a corrupted or non-deterministic report, and the build and release
workflows under `.github/workflows/`.

Out of scope: incorrect indicator mathematics (a functional bug, not a
vulnerability), and vulnerabilities in third-party dependencies — report those
upstream; we track them with Dependabot, `cargo-deny` and `osv-scanner`.

## Security assurance case

This is a short, evidence-backed argument for why the screener can be used
safely.

**Security requirements.** The screener is a computational component: it folds
numeric market data through indicators resolved by name and answers a boolean
condition tree. It stores no user credentials, authenticates no external users,
and implements no cryptography of its own. The requirements are therefore
(1) memory safety and freedom from undefined behaviour, (2) robust handling of
untrusted or degenerate input without panics or unbounded resource use,
(3) integrity of the published artifacts, and (4) a healthy dependency supply
chain.

**How the requirements are met.**

- *Memory safety* — the core and every binding are written in Rust, and the
  workspace sets `unsafe_code = "forbid"`. The one exception is the C ABI
  ([`bindings/c`](bindings/c)), whose thin FFI shim is necessarily `unsafe`
  because it dereferences caller-supplied pointers. It adds no scan logic,
  checks every handle and string for NULL, validates UTF-8, and catches panics
  at the boundary rather than letting one unwind into a foreign runtime — so the
  safe core's guarantees still cover all computation.
- *Input robustness* — a `ScanSpec` is validated before a scan begins, including
  rejecting a spec whose indicators need a feed the caller cannot supply; a
  division that is not finite yields no value rather than a surprise; and the
  JSON command envelope, the spec parser and the feed conversions are exercised
  by coverage-guided fuzzing (`cargo-fuzz` / libFuzzer) in CI.
- *Determinism across boundaries* — the golden corpus holds every binding to
  byte-identical output for the same spec and universe, and a streaming
  equivalence test in each of the nine reaches proves that feeding bar by bar
  matches a single batch scan. That is what turns "the same result in ten
  languages" into a checked property rather than a claim.
- *Static and dynamic analysis* — every push and pull request runs Clippy
  (`clippy::pedantic`, warnings as errors) under both feature sets, CodeQL
  across every binding language, zizmor and actionlint over the workflows,
  fuzzing, and the full test suite, with coverage on the core crate tracked by
  Codecov.
- *Artifact integrity* — releases are built in CI, commits and tags are signed,
  and release artifacts carry build provenance attestations covering the crates,
  wheels, npm tarballs, the NuGet package, the jar and the C ABI archives.
- *Supply chain* — dependencies are pinned and monitored with Dependabot, and
  audited with `cargo-deny` (licence + advisory policy) and `osv-scanner` (which
  also reads the npm, Go, Maven and NuGet manifests the bindings carry) on every
  change. CI's own Python tooling is installed from hash-pinned lock files, and
  every GitHub Action is pinned by commit SHA.

**Residual risk.** The optional `live` feature opens a TLS connection to an
exchange through the platform TLS library, so transport security there depends on
that library rather than on this project. It reads public market data only: no
key is sent and no order is placed. The screener is not a trading system and is
provided "as is" — see the disclaimers in `README.md` and the licences.

## Secrets management

The project stores **no** secrets or credentials in version control. Secrets
required by automation (the publishing tokens) are kept exclusively as
**GitHub Actions encrypted secrets** at the organisation level and referenced
through the `secrets.*` context; they are never written to the repository, to
logs, or to build artifacts. NuGet publishing uses OIDC and needs no stored
token at all. GitHub **secret scanning with push protection** is enabled to block
an accidental commit of a credential. Secrets follow least privilege — the
narrowest scope that works — and are rotated when a holder changes or on
suspected exposure.

## Verifying releases

Once a release exists, its artifacts can be verified for integrity and
authenticity:

- **Build provenance.** Release assets carry GitHub build provenance
  attestations. Verify a downloaded asset with the GitHub CLI:
  `gh attestation verify <file> --repo wickra-lib/wickra-screener`.
- **Signed tags.** Each release corresponds to a signed git tag (`vX.Y.Z`); the
  tag signature identifies the maintainer who authorised the release.
- **Registry integrity.** Packages are distributed over HTTPS from crates.io,
  PyPI, npm, NuGet and Maven Central, which serve checksums that package
  managers verify on install.

The release is published only by the maintainer through the tag-triggered release
workflow — a `workflow_dispatch` from a branch is refused by a guard job — so a
verified tag signature establishes the expected publisher identity.

## Support timeline and end of support

Only the **latest released version** receives security fixes. When a newer
release is published, the previous version **immediately reaches end of support**
and will not receive further fixes; users should upgrade. The supported-versions
table above is authoritative. A defined support window covering older releases
may be introduced later; until it is, only the latest release is supported.

## Remediation policy (dependencies and code scanning)

- **Severity threshold.** Vulnerabilities of **medium severity or higher** in the
  project's own code or its dependencies are remediated promptly and before the
  next release; lower-severity findings are addressed on a best-effort basis.
- **Automated enforcement (SCA).** Every change is evaluated by `cargo-deny`
  (RUSTSEC advisories + licence policy), `osv-scanner` and Dependabot. A known
  vulnerable dependency fails CI and **blocks the change** until it is resolved
  or explicitly waived with a written justification.
- **Automated enforcement (SAST).** Every change is evaluated by CodeQL across
  every binding language and by Clippy (`-D warnings`); findings **block the
  change** in CI until they are fixed or dismissed with a recorded reason.
- **Pre-release gate.** A release is not cut while an unresolved medium-or-higher
  SCA or SAST finding is outstanding.

## Vulnerability exploitability (VEX)

This repository ships a machine-readable VEX record in
[`osv-scanner.toml`](osv-scanner.toml), kept in lock-step with the cargo-deny
advisory ignore list in [`deny.toml`](deny.toml). Any advisory assessed as not
affecting `wickra-screener` is documented there with a reason — including which
code path is unreachable or which feature is not enabled — so downstream scanners
see an explicit, auditable justification rather than an unexplained suppression,
and no unnecessary dependency bump is forced.
