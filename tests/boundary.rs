use buntrail_ffi::*;
use std::ffi::{CStr, CString};

#[test]
fn ffi_returns_owned_errors_and_survives_malformed_input() {
    unsafe {
        let engine = bt_engine_new();
        assert!(!engine.is_null());
        assert_eq!(bt_abi_version(), 1);
        let request = CString::new("not JSON").unwrap();
        let response = bt_call(engine, request.as_ptr());
        let parsed: serde_json::Value =
            serde_json::from_slice(CStr::from_ptr(response).to_bytes()).unwrap();
        assert_eq!(parsed["ok"], false);
        bt_string_free(response);
        let request = CString::new(r#"{"command":"document.open","text":"rabbit 🐇"}"#).unwrap();
        let response = bt_call(engine, request.as_ptr());
        let parsed: serde_json::Value =
            serde_json::from_slice(CStr::from_ptr(response).to_bytes()).unwrap();
        assert_eq!(parsed["value"]["text"], "rabbit 🐇");
        bt_string_free(response);
        bt_string_free(std::ptr::null_mut());
        bt_engine_free(engine);
    }
}

#[test]
fn null_handles_and_null_requests_return_errors() {
    unsafe {
        let response = bt_call(std::ptr::null_mut(), std::ptr::null());
        assert!(!response.is_null());
        assert!(
            CStr::from_ptr(response)
                .to_str()
                .unwrap()
                .contains("invalid_argument")
        );
        bt_string_free(response);
        bt_engine_free(std::ptr::null_mut());
    }
}
