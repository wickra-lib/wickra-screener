# Security Policy

`wickra-screener` is analysis software: it evaluates data-driven conditions over
market data and places no orders. It holds no order-secret material and opens no
authenticated connections on its default path (the optional `live` universe
feature only reads public data). The attack surface is therefore narrow —
principally the parsing of untrusted `ScanSpec` / universe data as it crosses the
C ABI and WASM boundary. See [THREAT_MODEL.md](THREAT_MODEL.md) for the asset
inventory and trust boundaries.

## Supported versions

This project is pre-release. Security fixes target the `main` branch and the most
recent published version once a release exists.

| Version | Supported |
|---------|-----------|
| `main`  | ✅        |
| `0.1.x` (upcoming) | ✅ |

## Reporting a vulnerability

**Please do not open a public issue, pull request or discussion for security
problems.** Report privately through either channel:

- GitHub → the repository's **Security** tab → **Report a vulnerability**
  (private advisory), or
- email **support@wickra.org**.

Include a description, affected version/commit, reproduction steps and impact.

We aim to acknowledge within a few days, agree a disclosure timeline, and credit
reporters who wish to be named once a fix ships.

## Scope

In scope: memory-safety or panic-across-FFI flaws in the C ABI hub and its
buffer protocol, denial-of-service through a hostile `ScanSpec` or dataset (for
example unbounded allocation while parsing), and any input that makes a binding
return a corrupted or non-deterministic report. Out of scope: incorrect indicator
mathematics (a functional bug, not a vulnerability) and advisories in third-party
crates that are already tracked and triaged.

## Vulnerability disclosure (VEX)

This repository ships a machine-readable VEX record in
[`osv-scanner.toml`](osv-scanner.toml), kept in lock-step with the cargo-deny
advisory ignore list in [`deny.toml`](deny.toml). Any advisory assessed as not
affecting `wickra-screener` is documented there with a reason, so downstream
scanners see an explicit, auditable justification rather than an unexplained
suppression.
