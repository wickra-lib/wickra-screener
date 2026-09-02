//! The wickra-screener C ABI — the hub every C-capable language links against.
//!
//! The surface is deliberately tiny and JSON-shaped, exactly like
//! `Screener::command_json`: construct a `Screener` from a spec JSON, drive it
//! with command JSONs and read back response JSONs, and free the handle. No
//! screener type crosses the boundary by value — the handle is opaque and the
//! payloads are always UTF-8 JSON strings.
//!
//! Responses use a caller-owned buffer with a length-out protocol (the classic
//! C two-call idiom), so the caller never has to free a callee allocation:
//!
//! 1. Call [`wickra_screener_command`] with `out = NULL`, `cap = 0` to learn the
//!    response length `len` (excluding the terminating NUL).
//! 2. Allocate `len + 1` bytes and call again; the response plus a NUL is
//!    written into `out`.
//!
//! Whenever `len < cap` the response is written immediately, so a
//! sufficiently-large buffer needs only one call. A command runs once per
//! *delivered* response: a body produced for a length query, or for a buffer
//! that turned out too small, is held and handed to the call that reads it. Negative returns are reserved
//! for unusable arguments ([`WICKRA_SCREENER_ERR_NULL`],
//! [`WICKRA_SCREENER_ERR_UTF8`]) and caught panics
//! ([`WICKRA_SCREENER_ERR_PANIC`]); a non-negative return is always the response
//! length. Domain errors (a bad spec, an unknown command) are *not* negative —
//! they come back in-band as `{"ok":false,"error":...}` JSON in the buffer.

use core::ffi::{c_char, CStr};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use screener_core::Screener;

/// A required pointer argument (`handle` or `cmd_json`) was null.
pub const WICKRA_SCREENER_ERR_NULL: i32 = -1;
/// `cmd_json` was not valid UTF-8.
pub const WICKRA_SCREENER_ERR_UTF8: i32 = -2;
/// A panic was caught at the FFI boundary.
pub const WICKRA_SCREENER_ERR_PANIC: i32 = -3;

/// An opaque handle to a screener instance. Created by [`wickra_screener_new`]
/// and destroyed by [`wickra_screener_free`]; never dereferenced by the caller.
///
/// Besides the screener it carries a response that was produced but not yet
/// delivered, which is what lets the two-call idiom run a command once rather
/// than twice. See [`wickra_screener_command`].
pub struct WickraScreener {
    screener: Screener,
    pending: Option<(String, String)>,
}

/// Read a NUL-terminated C string as `&str`, or `None` on null / bad UTF-8.
///
/// # Safety
/// `ptr` must be null or a valid NUL-terminated C string.
unsafe fn opt_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr) }.to_str().ok()
}

/// Construct a screener from a spec JSON string.
///
/// Returns an opaque handle, or null if `spec_json` is null, not valid UTF-8, or
/// not a valid spec. Free the handle with [`wickra_screener_free`].
///
/// # Safety
/// `spec_json` must be null or a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn wickra_screener_new(spec_json: *const c_char) -> *mut WickraScreener {
    let Some(json) = (unsafe { opt_str(spec_json) }) else {
        return ptr::null_mut();
    };
    match catch_unwind(AssertUnwindSafe(|| Screener::new(json))) {
        Ok(Ok(screener)) => Box::into_raw(Box::new(WickraScreener {
            screener,
            pending: None,
        })),
        _ => ptr::null_mut(),
    }
}

/// Destroy a screener handle. Null is a no-op.
///
/// # Safety
/// `handle` must be null or a handle previously returned by
/// [`wickra_screener_new`] and not already freed.
#[no_mangle]
pub unsafe extern "C" fn wickra_screener_free(handle: *mut WickraScreener) {
    if !handle.is_null() {
        drop(unsafe { Box::from_raw(handle) });
    }
}

/// Apply a command JSON and write the response JSON into the caller's buffer.
///
/// Returns the response length in bytes (excluding the terminating NUL), or a
/// negative error code. When the return value `len` satisfies `len < cap`, the
/// response and a trailing NUL have been written to `out`; otherwise `out` is
/// left untouched and the caller should re-call with a `cap` of at least
/// `len + 1`. Pass `out = NULL`, `cap = 0` to query the length without writing.
///
/// The command runs **once per delivered response**, not once per call. A
/// response that was produced but not written -- a length query, or a buffer too
/// small -- is held until the call that reads it, and that call returns it
/// without running the command again. This matters for every command that
/// mutates: before it, `feed` applied each candle twice in Go, C, C++, C#, Java
/// and R, all of which use the two-call idiom, while `scan` looked correct
/// because it is a pure function of its payload.
///
/// # Safety
/// `handle` must be a valid handle; `cmd_json` a valid NUL-terminated C string;
/// `out` either null or a writable buffer of at least `cap` bytes.
#[no_mangle]
pub unsafe extern "C" fn wickra_screener_command(
    handle: *mut WickraScreener,
    cmd_json: *const c_char,
    out: *mut c_char,
    cap: usize,
) -> i32 {
    if handle.is_null() || cmd_json.is_null() {
        return WICKRA_SCREENER_ERR_NULL;
    }
    let Some(cmd) = (unsafe { opt_str(cmd_json) }) else {
        return WICKRA_SCREENER_ERR_UTF8;
    };
    let state = unsafe { &mut *handle };

    // A response already produced for this exact command and not yet delivered.
    // Without this the length-query call and the too-small-buffer retry each run
    // the command again -- harmless for `scan`, which is a pure function of its
    // payload, and wrong for every command that mutates: `feed` applied each
    // candle twice in every language that uses the two-call idiom.
    let carried = matches!(&state.pending, Some((pending_cmd, _)) if pending_cmd == cmd);
    let response = if carried {
        // `carried` proves the entry is present, so the take cannot be None.
        state.pending.take().expect("pending response present").1
    } else {
        // A different command abandons whatever was queued: the caller moved on,
        // and serving that stale body later would skip an execution it wants.
        state.pending = None;
        let screener = &mut state.screener;
        match catch_unwind(AssertUnwindSafe(|| screener.command_json(cmd))) {
            // `command_json` folds domain errors into `{"ok":false,...}` JSON, so
            // a top-level `Err` should not occur; surface it in-band all the same
            // rather than inventing a new negative code.
            Ok(result) => result.unwrap_or_else(|err| {
                format!(
                    "{{\"ok\":false,\"error\":{}}}",
                    json_string(&err.to_string())
                )
            }),
            Err(_) => return WICKRA_SCREENER_ERR_PANIC,
        }
    };

    let bytes = response.as_bytes();
    let len = bytes.len();
    if len < cap && !out.is_null() {
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), out.cast::<u8>(), len);
            *out.add(len) = 0;
        }
    } else {
        // Produced but not delivered: keep it for the call that reads it, so the
        // command behind it runs exactly once.
        state.pending = Some((cmd.to_string(), response));
    }
    i32::try_from(len).unwrap_or(i32::MAX)
}

/// The library version as a static NUL-terminated string (do not free).
#[no_mangle]
pub extern "C" fn wickra_screener_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0")
        .as_ptr()
        .cast::<c_char>()
}

/// Encode a string as a JSON string literal (quotes + minimal escaping).
fn json_string(s: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    const SPEC: &str = r#"{"universe":["AAA","BBB"],"condition":{"type":"cmp","left":{"kind":"price","field":"close"},"op":"gt","right":{"kind":"const","value":10.0}}}"#;

    /// A spec whose condition carries a lookback. `SPEC` compares the close
    /// against a constant, so it reads the same however many times a bar was
    /// fed -- which is why the first version of the tests below passed with the
    /// fix removed. `Sma(3)` over 10, 20, 30 is 20; fed twice each it is the
    /// mean of 20, 30, 30, and the difference is the whole point.
    const SPEC_WITH_HISTORY: &str = r#"{"universe":["AAA"],"condition":{"type":"cmp","left":{"kind":"indicator","name":"Sma","params":[3]},"op":"gt","right":{"kind":"const","value":0.0}}}"#;

    /// One bar of the 10 / 20 / 30 ramp.
    fn bar(time: i64, close: f64) -> String {
        format!(
            r#"{{"cmd":"feed","symbol":"AAA","candle":{{"time":{time},"open":{close},"high":{close},"low":{close},"close":{close},"volume":1.0}}}}"#
        )
    }

    /// Read a NUL-terminated buffer written by the command call as a `String`.
    fn read_buf(buf: &[u8]) -> String {
        let cstr = CStr::from_bytes_until_nul(buf).unwrap();
        cstr.to_str().unwrap().to_string()
    }

    /// Drive one command through the documented two-call idiom and return the
    /// response: query the length with a null buffer, then read it.
    fn two_call(handle: *mut WickraScreener, cmd: &str) -> String {
        let cmd = CString::new(cmd).unwrap();
        let len = unsafe { wickra_screener_command(handle, cmd.as_ptr(), ptr::null_mut(), 0) };
        assert!(len >= 0, "length query failed: {len}");
        let mut buf = vec![0u8; len as usize + 1];
        let written = unsafe {
            wickra_screener_command(handle, cmd.as_ptr(), buf.as_mut_ptr().cast(), buf.len())
        };
        assert_eq!(written, len);
        read_buf(&buf)
    }

    /// The two-call idiom must apply a mutating command once, not twice.
    ///
    /// Every reach behind this ABI -- Go, C, C++, C#, Java, R -- asks for the
    /// length first and reads second. The command used to run on both calls, so
    /// each `feed` applied its candle twice and every indicator saw a doubled
    /// history. `scan` hid it: it is a pure function of its payload, and the
    /// golden corpus sends nothing else.
    #[test]
    fn a_mutating_command_runs_once_across_the_two_call_idiom() {
        let spec = CString::new(SPEC_WITH_HISTORY).unwrap();
        let handle = unsafe { wickra_screener_new(spec.as_ptr()) };
        assert!(!handle.is_null());

        for (time, close) in [(1i64, 10.0f64), (2, 20.0), (3, 30.0)] {
            two_call(handle, &bar(time, close));
        }
        let via_two_call = two_call(handle, r#"{"cmd":"evaluate"}"#);

        // Sma(3) over 10, 20, 30. Fed twice each it would be the mean of
        // 20, 30, 30 instead, so the value is what distinguishes the two.
        assert!(
            via_two_call.contains("\"Sma(3)\":20.0"),
            "expected Sma(3) = 20.0 from three bars, got: {via_two_call}"
        );

        unsafe { wickra_screener_free(handle) };
    }

    /// A buffer too small also produces a response that is not delivered. The
    /// retry must read that body, not run the command a second time.
    #[test]
    fn a_too_small_buffer_does_not_rerun_the_command() {
        let spec = CString::new(SPEC_WITH_HISTORY).unwrap();
        let handle = unsafe { wickra_screener_new(spec.as_ptr()) };

        for (time, close) in [(1i64, 10.0f64), (2, 20.0), (3, 30.0)] {
            let cmd = CString::new(bar(time, close)).unwrap();
            // One byte of capacity: the response cannot fit, so nothing is
            // written and the caller must retry, exactly as the docs prescribe.
            let mut tiny = [0u8; 1];
            let len = unsafe {
                wickra_screener_command(handle, cmd.as_ptr(), tiny.as_mut_ptr().cast(), tiny.len())
            };
            assert!(len >= 1, "response should be longer than the buffer");
            let mut buf = vec![0u8; len as usize + 1];
            let written = unsafe {
                wickra_screener_command(handle, cmd.as_ptr(), buf.as_mut_ptr().cast(), buf.len())
            };
            assert_eq!(written, len);
        }

        let after_retries = two_call(handle, r#"{"cmd":"evaluate"}"#);
        assert!(
            after_retries.contains("\"Sma(3)\":20.0"),
            "the truncation retry fed each candle a second time: {after_retries}"
        );

        unsafe { wickra_screener_free(handle) };
    }

    /// Moving to a different command abandons a queued body, so a later query
    /// for the first one runs it again instead of serving a stale response.
    #[test]
    fn a_different_command_abandons_the_queued_response() {
        let spec = CString::new(SPEC_WITH_HISTORY).unwrap();
        let handle = unsafe { wickra_screener_new(spec.as_ptr()) };

        for (time, close) in [(1i64, 10.0f64), (2, 20.0)] {
            two_call(handle, &bar(time, close));
        }

        // Query the length of the third bar, then never read it. That call is a
        // request to run it, so it runs -- once.
        let third = CString::new(bar(3, 30.0)).unwrap();
        let len = unsafe { wickra_screener_command(handle, third.as_ptr(), ptr::null_mut(), 0) };
        assert!(len >= 0);

        // A different command in between drops the queued body.
        let version = two_call(handle, r#"{"cmd":"version"}"#);
        assert!(version.contains("version"));

        let after = two_call(handle, r#"{"cmd":"evaluate"}"#);
        assert!(
            after.contains("\"Sma(3)\":20.0"),
            "the abandoned query should have fed the third bar exactly once: {after}"
        );

        unsafe { wickra_screener_free(handle) };
    }

    #[test]
    fn new_command_free_round_trip() {
        let spec = CString::new(SPEC).unwrap();
        let handle = unsafe { wickra_screener_new(spec.as_ptr()) };
        assert!(!handle.is_null());

        let cmd = CString::new(r#"{"cmd":"version"}"#).unwrap();
        // First call: query the length with a null buffer.
        let len = unsafe { wickra_screener_command(handle, cmd.as_ptr(), ptr::null_mut(), 0) };
        assert!(len > 0);

        // Second call: allocate len + 1 and read the response back.
        let mut buf = vec![0u8; usize::try_from(len).unwrap() + 1];
        let len2 = unsafe {
            wickra_screener_command(
                handle,
                cmd.as_ptr(),
                buf.as_mut_ptr().cast::<c_char>(),
                buf.len(),
            )
        };
        assert_eq!(len2, len);
        let response = read_buf(&buf);
        assert!(response.contains("\"version\""));

        unsafe { wickra_screener_free(handle) };
    }

    #[test]
    fn too_small_buffer_leaves_out_untouched() {
        let spec = CString::new(SPEC).unwrap();
        let handle = unsafe { wickra_screener_new(spec.as_ptr()) };
        let cmd = CString::new(r#"{"cmd":"version"}"#).unwrap();

        let mut buf = vec![0xAAu8; 4]; // deliberately too small
        let len = unsafe {
            wickra_screener_command(
                handle,
                cmd.as_ptr(),
                buf.as_mut_ptr().cast::<c_char>(),
                buf.len(),
            )
        };
        assert!(usize::try_from(len).unwrap() >= buf.len());
        assert!(buf.iter().all(|&b| b == 0xAA)); // untouched

        unsafe { wickra_screener_free(handle) };
    }

    #[test]
    fn bad_command_reports_error_in_band() {
        let spec = CString::new(SPEC).unwrap();
        let handle = unsafe { wickra_screener_new(spec.as_ptr()) };
        let bad = CString::new(r#"{"cmd":"nope"}"#).unwrap();

        let len = unsafe { wickra_screener_command(handle, bad.as_ptr(), ptr::null_mut(), 0) };
        assert!(len > 0); // in-band error, not a negative code
        let mut buf = vec![0u8; usize::try_from(len).unwrap() + 1];
        unsafe {
            wickra_screener_command(
                handle,
                bad.as_ptr(),
                buf.as_mut_ptr().cast::<c_char>(),
                buf.len(),
            );
        }
        assert!(read_buf(&buf).contains("\"ok\":false"));

        unsafe { wickra_screener_free(handle) };
    }

    #[test]
    fn null_spec_yields_null_handle() {
        let handle = unsafe { wickra_screener_new(ptr::null()) };
        assert!(handle.is_null());
    }

    #[test]
    fn null_guards_on_command() {
        let cmd = CString::new(r#"{"cmd":"version"}"#).unwrap();
        // Null handle.
        let code =
            unsafe { wickra_screener_command(ptr::null_mut(), cmd.as_ptr(), ptr::null_mut(), 0) };
        assert_eq!(code, WICKRA_SCREENER_ERR_NULL);
        // Null command with a valid handle.
        let spec = CString::new(SPEC).unwrap();
        let handle = unsafe { wickra_screener_new(spec.as_ptr()) };
        let code = unsafe { wickra_screener_command(handle, ptr::null(), ptr::null_mut(), 0) };
        assert_eq!(code, WICKRA_SCREENER_ERR_NULL);
        unsafe { wickra_screener_free(handle) };
    }

    #[test]
    fn free_null_is_a_noop() {
        unsafe { wickra_screener_free(ptr::null_mut()) };
    }

    #[test]
    fn version_is_nul_terminated() {
        let p = wickra_screener_version();
        let v = unsafe { CStr::from_ptr(p) }.to_str().unwrap();
        assert_eq!(v, env!("CARGO_PKG_VERSION"));
    }
}
