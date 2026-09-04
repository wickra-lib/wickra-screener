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

cat("wickra-screener R tests passed\n")
