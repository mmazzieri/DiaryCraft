//! [`crate::call`] for C and Swift.
//!
//! ```c
//! char *diarycraft_call(const char *request);   // JSON in, JSON out
//! void diarycraft_free(char *answer);           // every answer, once
//! ```
//!
//! The header is `mobile/ios/DiaryCraft/DiaryCraftCore.h`.
#![allow(unsafe_code)]

use std::ffi::{CStr, CString, c_char};

/// Runs one request. `request` is a NUL-terminated UTF-8 string; a null pointer or text that is
/// not UTF-8 is answered with an error. The answer is never null and is given back to
/// [`diarycraft_free`].
///
/// # Safety
///
/// `request` is null or points to a NUL-terminated string that stays valid during the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn diarycraft_call(request: *const c_char) -> *mut c_char {
    let input = if request.is_null() {
        None
    } else {
        // SAFETY: not null, and the caller promises a NUL-terminated string that outlives the call.
        unsafe { CStr::from_ptr(request) }.to_str().ok()
    };
    let answer = match input {
        Some(text) => crate::call(text),
        None => "{\"error\":\"the request is not UTF-8 text\"}".to_owned(),
    };
    // JSON from serde_json escapes NUL as \u0000, so the answer has none inside.
    CString::new(answer).map_or(std::ptr::null_mut(), CString::into_raw)
}

/// Frees an answer of [`diarycraft_call`]. A null pointer is ignored.
///
/// # Safety
///
/// `answer` is null or a pointer `diarycraft_call` returned and that was not freed before.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn diarycraft_free(answer: *mut c_char) {
    if !answer.is_null() {
        // SAFETY: the pointer came from `CString::into_raw` in `diarycraft_call` and is freed once.
        drop(unsafe { CString::from_raw(answer) });
    }
}
