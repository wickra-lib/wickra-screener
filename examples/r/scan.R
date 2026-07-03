# A runnable R example: scan a small universe through the binding.
#
#   cargo build -p wickra-screener-c --release
#   export WKSCREEN_INC="$PWD/bindings/c/include"
#   export WKSCREEN_LIB="$PWD/target/release"
#   export LD_LIBRARY_PATH="$WKSCREEN_LIB:$LD_LIBRARY_PATH"   # PATH on Windows
#   R CMD INSTALL bindings/r
#   Rscript examples/r/scan.R

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

screener <- wkscreen_new(spec)
cmd <- paste0(
  '{"cmd":"scan","data":{',
  '"AAA":[', candle(5), '],',
  '"BBB":[', candle(15), ']}}'
)
response <- wkscreen_command(screener, cmd)

cat("wickra-screener", wkscreen_version(), "\n")
cat(response, "\n")
