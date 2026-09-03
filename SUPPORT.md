# Support

Thanks for using `wickra-screener`. Here is where to get help, depending on what
you need.

## Documentation first

Most questions are answered in the documentation:

- **Guides:** [`docs/`](docs/) — the shape of a `ScanSpec`
  ([CONDITIONS](docs/CONDITIONS.md)), naming an indicator and its feed
  ([INDICATORS](docs/INDICATORS.md)), rank / percentile / z-score and breadth
  ([CROSS_SECTION](docs/CROSS_SECTION.md)), batch against streaming
  ([STREAMING](docs/STREAMING.md)), worked screens
  ([Cookbook](docs/Cookbook.md)), and the internals
  ([ARCHITECTURE](docs/ARCHITECTURE.md)).
- **README:** <https://github.com/wickra-lib/wickra-screener#readme> —
  installation and a quick overview.
- **Site:** <https://screener.wickra.org> — the in-browser demo and the
  benchmark figures.
- **Examples:** [`examples/`](examples/) — one runnable program per language,
  each built and run in CI, so what is written there works.
- **Indicator reference:** <https://docs.wickra.org> — the screener resolves
  indicators by name from the Wickra library, which documents each one there.

## Questions and help

- Open a [GitHub Discussion](https://github.com/wickra-lib/wickra-screener/discussions)
  for questions and ideas, or ask with the
  [question issue template](.github/ISSUE_TEMPLATE/question.md).
- Browse [existing issues](https://github.com/wickra-lib/wickra-screener/issues) —
  your question may already be answered.

## Bugs and feature requests

- **Bugs:** use the bug-report issue template.
- **Feature requests:** use the feature-request template.

Please include the version, the binding or language you used, a minimal
`ScanSpec` and a small sample universe, and the expected versus actual report.
A spec and a handful of candles that reproduce the problem are worth more than a
description of it.

## Security issues

Please do **not** report security vulnerabilities through public issues. Follow
the process in [`SECURITY.md`](SECURITY.md) — a private GitHub advisory or email
to **support@wickra.org**.

## Support expectations

`wickra-screener` is maintained by a single maintainer on a best-effort basis.
Issues are triaged and acknowledged as time allows; there is no commercial
support or SLA. Clear, reproducible reports get help fastest.

## Note

`wickra-screener` is a research and engineering tool: it evaluates conditions
over market data and places no orders. It is not financial advice and comes with
no warranty — review the code and validate results before relying on them.
