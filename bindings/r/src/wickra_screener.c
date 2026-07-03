/* R .Call glue for the wickra-screener C ABI hub. */
#include <R.h>
#include <Rinternals.h>
#include <R_ext/Rdynload.h>
#include <stddef.h>
#include "wickra_screener.h"

/* --- handle lifetime ----------------------------------------------------- */

static void wkscreen_finalize(SEXP ext) {
    WickraScreener *h = (WickraScreener *)R_ExternalPtrAddr(ext);
    if (h) {
        wickra_screener_free(h);
    }
    R_ClearExternalPtr(ext);
}

static WickraScreener *handle_of(SEXP ext) {
    WickraScreener *h = (WickraScreener *)R_ExternalPtrAddr(ext);
    if (!h) {
        Rf_error("wickra-screener: handle is closed");
    }
    return h;
}

/* --- exported .Call entries ---------------------------------------------- */

SEXP wkscreen_version(void) {
    return Rf_mkString(wickra_screener_version());
}

SEXP wkscreen_new(SEXP spec_json) {
    WickraScreener *h = wickra_screener_new(CHAR(STRING_ELT(spec_json, 0)));
    if (!h) {
        Rf_error("wickra-screener: invalid spec");
    }
    SEXP ext = PROTECT(R_MakeExternalPtr(h, R_NilValue, R_NilValue));
    R_RegisterCFinalizerEx(ext, wkscreen_finalize, TRUE);
    UNPROTECT(1);
    return ext;
}

SEXP wkscreen_command(SEXP ext, SEXP cmd_json) {
    WickraScreener *h = handle_of(ext);
    const char *cmd = CHAR(STRING_ELT(cmd_json, 0));

    /* Length-out protocol: learn the length, then read into a caller buffer.
       Domain errors come back in-band as {"ok":false,...} JSON, not a negative
       code; only unusable arguments / a caught panic return < 0. */
    int len = wickra_screener_command(h, cmd, NULL, 0);
    if (len < 0) {
        Rf_error("wickra-screener: command failed (code %d)", len);
    }
    char *buf = (char *)R_alloc((size_t)len + 1, 1);
    wickra_screener_command(h, cmd, buf, (size_t)len + 1);
    return Rf_mkString(buf);
}

/* --- registration -------------------------------------------------------- */

static const R_CallMethodDef CallEntries[] = {
    {"wkscreen_version", (DL_FUNC)&wkscreen_version, 0},
    {"wkscreen_new", (DL_FUNC)&wkscreen_new, 1},
    {"wkscreen_command", (DL_FUNC)&wkscreen_command, 2},
    {NULL, NULL, 0}};

void R_init_wickrascreener(DllInfo *dll) {
    R_registerRoutines(dll, NULL, CallEntries, NULL, NULL);
    R_useDynamicSymbols(dll, FALSE);
}
