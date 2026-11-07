//! Native C ABI for the Buntrail engine.
//!
//! See the repository README for usage, prerequisites and platform limitations.
//! The only exported native boundary. Returned strings have explicit ownership.
mod editor;
use buntrail_engine::Engine;
pub use editor::*;
use std::ffi::{CStr, CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct BtEngine {
    engine: Engine,
    failed: AtomicBool,
    identity: std::sync::Arc<()>,
}

#[unsafe(no_mangle)]
pub extern "C" fn bt_abi_version() -> u32 {
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn bt_engine_new() -> *mut BtEngine {
    catch_unwind(|| {
        Box::into_raw(Box::new(BtEngine {
            engine: Engine::new(),
            failed: AtomicBool::new(false),
            identity: std::sync::Arc::new(()),
        }))
    })
    .unwrap_or(std::ptr::null_mut())
}

/// # Safety
/// `engine` must be null or an engine returned by bt_engine_new, freed exactly
/// once, after every call using it has returned.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bt_engine_free(engine: *mut BtEngine) {
    if !engine.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| drop(unsafe { Box::from_raw(engine) })));
    }
}

fn owned_error(code: &str, message: &str) -> *mut c_char {
    owned(serde_json::json!({"ok":false,"error":{"code":code,"message":message}}).to_string())
}
fn owned(text: String) -> *mut c_char {
    // JSON serialization escapes NUL characters, so the string is a valid C string.
    CString::new(text)
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

/// Calls are thread-safe; UI code serializes composition/edit transactions.
/// No GUI callback is invoked. Perform service calls on a worker thread.
/// # Safety
/// `engine` must be live. `request` must be a readable NUL-terminated string
/// throughout this call. Neither pointer may be freed concurrently.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bt_call(engine: *mut BtEngine, request: *const c_char) -> *mut c_char {
    if engine.is_null() || request.is_null() {
        return owned_error("invalid_argument", "Engine and request are required");
    }
    let engine = unsafe { &*engine };
    if engine.failed.load(Ordering::Acquire) {
        return owned_error("engine_failed", "Engine failed; preserve drafts and reopen");
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        let bytes = unsafe { CStr::from_ptr(request) }.to_bytes();
        if bytes.len() > 128 * 1024 * 1024 {
            return owned_error("too_large", "Request exceeds 128 MiB");
        }
        let request = match serde_json::from_slice(bytes) {
            Ok(value) => value,
            Err(error) => return owned_error("invalid_json", &error.to_string()),
        };
        owned(engine.engine.call(request).to_string())
    }));
    result.unwrap_or_else(|_| {
        engine.failed.store(true, Ordering::Release);
        owned_error(
            "engine_failed",
            "Rust panic contained at ABI; this engine cannot accept further work",
        )
    })
}

/// # Safety
/// `value` must be null or a string returned by bt_call, freed exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bt_string_free(value: *mut c_char) {
    if !value.is_null() {
        drop(unsafe { CString::from_raw(value) });
    }
}
