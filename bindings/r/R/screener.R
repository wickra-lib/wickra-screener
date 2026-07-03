#' The wickra-screener library version.
#' @return A version string.
#' @export
wkscreen_version <- function() {
  .Call(C_wkscreen_version)
}

#' Build a screener from a spec JSON string.
#' @param spec_json A JSON spec string.
#' @return A `wickra_screener` handle (an external pointer).
#' @export
wkscreen_new <- function(spec_json) {
  .Call(C_wkscreen_new, spec_json)
}

#' Apply a command JSON and return the resulting response JSON.
#' @param screener A screener handle from [wkscreen_new()].
#' @param cmd_json A command JSON string.
#' @return The response as a JSON string.
#' @export
wkscreen_command <- function(screener, cmd_json) {
  .Call(C_wkscreen_command, screener, cmd_json)
}
