## Plain-R tests for the wickra-screener R binding (no testthat dependency).
## Mirrors the Rust/Python/Node/Go/C#/Java tests and doubles as the completeness
## guard: it exercises the full public surface (version + new + command).

library(wickrascreener)

spec <- paste0(
  '{"universe":["AAA","BBB"],"condition":{"type":"cmp",',
  '"left":{"kind":"price","field":"close"},"op":"gt",',
  '"right":{"kind":"const","value":10.0}}}'
)

candle <- function(close) {
  paste0(
    '{"time":1,"open":', close, ',"high":', close,
    ',"low":', close, ',"close":', close, ',"volume":1}'
  )
}

## version
stopifnot(nzchar(wkscreen_version()))

## scan -> only BBB matches (close > 10)
screener <- wkscreen_new(spec)
cmd <- paste0(
  '{"cmd":"scan","data":{',
  '"AAA":[', candle(5), '],',
  '"BBB":[', candle(15), ']}}'
)
raw <- wkscreen_command(screener, cmd)
stopifnot(grepl('"scanned":2', raw, fixed = TRUE))
stopifnot(grepl('"symbol":"BBB"', raw, fixed = TRUE))
stopifnot(!grepl('"symbol":"AAA"', raw, fixed = TRUE))

## invalid spec raises
stopifnot(inherits(try(wkscreen_new("not json"), silent = TRUE), "try-error"))

## an unknown command is an in-band error, not a hard error
inband <- wkscreen_command(screener, '{"cmd":"nope"}')
stopifnot(grepl('"ok":false', inband, fixed = TRUE))

## cross-language golden parity: build the screener from each committed
## golden/specs/*.json, run a scan over the shared golden/data.json, and assert
## the response equals golden/expected/<spec>.json byte-for-byte. The binding
## returns the core's compact command output verbatim, so byte equality is the
## exact cross-language parity check. The fixtures arrive in a later phase; until
## then the golden section is skipped.
golden_dir <- function() {
  d <- normalizePath(getwd(), mustWork = FALSE)
  for (i in seq_len(8)) {
    g <- file.path(d, "golden")
    if (dir.exists(file.path(g, "specs"))) {
      return(g)
    }
    d <- dirname(d)
  }
  NULL
}

g <- golden_dir()
if (!is.null(g)) {
  dataset <- trimws(paste(
    readLines(file.path(g, "data.json"), warn = FALSE), collapse = "\n"
  ))
  for (spec_path in list.files(file.path(g, "specs"), pattern = "\\.json$", full.names = TRUE)) {
    name <- basename(spec_path)
    spec_json <- paste(readLines(spec_path, warn = FALSE), collapse = "\n")
    expected <- trimws(paste(
      readLines(file.path(g, "expected", name), warn = FALSE), collapse = "\n"
    ))
    gscreener <- wkscreen_new(spec_json)
    got <- wkscreen_command(gscreener, paste0('{"cmd":"scan","data":', dataset, '}'))
    stopifnot(identical(trimws(got), expected))
  }
}

cat("wickra-screener R tests passed\n")
