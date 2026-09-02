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
## golden/specs/*.json, run a scan over the matching committed dataset, and
## assert the response equals golden/expected/<spec>.json byte-for-byte. The
## binding returns the core's compact command output verbatim, so byte equality
## is the exact cross-language parity check. A spec named feeds_* scans
## data-feeds.json, which carries the side feeds; every other spec scans the
## candle-only data.json.
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
  read_dataset <- function(file) {
    trimws(paste(readLines(file.path(g, file), warn = FALSE), collapse = "\n"))
  }
  datasets <- list(
    "data.json" = read_dataset("data.json"),
    "data-feeds.json" = read_dataset("data-feeds.json")
  )
  for (spec_path in list.files(file.path(g, "specs"), pattern = "\\.json$", full.names = TRUE)) {
    name <- basename(spec_path)
    dataset <- datasets[[if (startsWith(name, "feeds_")) "data-feeds.json" else "data.json"]]
    spec_json <- paste(readLines(spec_path, warn = FALSE), collapse = "\n")
    expected <- trimws(paste(
      readLines(file.path(g, "expected", name), warn = FALSE), collapse = "\n"
    ))
    gscreener <- wkscreen_new(spec_json)
    got <- wkscreen_command(gscreener, paste0('{"cmd":"scan","data":', dataset, '}'))
    stopifnot(identical(trimws(got), expected))
  }
}
## streaming equals batch, driven through the same command boundary.
##
## screener-core proves this in Rust, but that says nothing about the boundary
## this binding crosses. The golden block above only ever sends {"cmd":"scan"},
## so feed and evaluate were exercised in no language at all -- which is how the
## C ABI shipped a command that ran twice under the two-call idiom this binding
## uses: scan is a pure function of its payload, so it looked correct while every
## feed applied its candle twice.
##
## Only the candle-only specs are used. A feeds_* spec needs side feeds the
## streaming envelope carries per bar, and derived_breadth needs the market
## panel, which a streaming screener cannot derive.
##
## base R carries no JSON parser and the binding takes no dependency for one. The
## fixture is machine-generated and each candle is a flat object, so matching
## brace runs without nested braces is enough; the counts are asserted below so a
## bad split fails loudly instead of comparing nothing.
##
## The fixtures are committed, so their absence is a broken checkout rather than
## a phase that has not arrived yet. Skipping here would compare nothing and
## report success.
if (is.null(g)) {
  stop("golden fixtures not found; the streaming comparison would test nothing")
}
{
  streaming_specs <- c(
    "momentum", "mean_reversion", "cross_section_rank",
    "breadth", "crossover", "compound"
  )
  data_text <- datasets[["data.json"]]

  key_hits <- regmatches(data_text, gregexpr('"[^"]+"[[:space:]]*:[[:space:]]*[[]', data_text))[[1]]
  symbol_names <- gsub('^"|"[[:space:]]*:[[:space:]]*[[]$', "", key_hits)
  stopifnot(length(symbol_names) >= 2)

  candle_of <- function(symbol) {
    start <- regexpr(paste0('"', symbol, '"[[:space:]]*:[[:space:]]*[[]'), data_text)
    stopifnot(start > 0)
    rest <- substring(data_text, start)
    close_at <- regexpr("[]]", rest)
    stopifnot(close_at > 0)
    body <- substring(rest, 1, close_at)
    regmatches(body, gregexpr("[{][^{}]*[}]", body))[[1]]
  }

  for (symbol in symbol_names) {
    stopifnot(length(candle_of(symbol)) >= 10)
  }

  feed_all <- function(screener) {
    for (symbol in symbol_names) {
      for (candle in candle_of(symbol)) {
        wkscreen_command(
          screener,
          paste0('{"cmd":"feed","symbol":"', symbol, '","candle":', candle, "}")
        )
      }
    }
  }

  compared <- 0
  for (name in streaming_specs) {
    spec_path <- file.path(g, "specs", paste0(name, ".json"))
    stopifnot(file.exists(spec_path))
    spec_json <- paste(readLines(spec_path, warn = FALSE), collapse = "\n")

    batch_screener <- wkscreen_new(spec_json)
    batch <- trimws(wkscreen_command(
      batch_screener, paste0('{"cmd":"scan","data":', data_text, "}")
    ))

    stream_screener <- wkscreen_new(spec_json)
    feed_all(stream_screener)
    streamed <- trimws(wkscreen_command(stream_screener, '{"cmd":"evaluate"}'))

    if (!identical(streamed, batch)) {
      stop(paste0("streaming != batch for spec ", name))
    }
    compared <- compared + 1
  }
  ## A loop that compared nothing passes; say how many it actually did.
  stopifnot(compared == length(streaming_specs))

  ## reset returns a screener to its pre-feed state
  spec_json <- paste(
    readLines(file.path(g, "specs", "momentum.json"), warn = FALSE), collapse = "\n"
  )
  rscreener <- wkscreen_new(spec_json)
  empty <- wkscreen_command(rscreener, '{"cmd":"evaluate"}')
  feed_all(rscreener)
  stopifnot(!identical(wkscreen_command(rscreener, '{"cmd":"evaluate"}'), empty))
  wkscreen_command(rscreener, '{"cmd":"reset"}')
  stopifnot(identical(wkscreen_command(rscreener, '{"cmd":"evaluate"}'), empty))
}

cat("wickra-screener R tests passed\n")
