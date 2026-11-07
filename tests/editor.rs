use buntrail_ffi::*;
use std::{
    ffi::{CStr, CString},
    ptr,
};
unsafe fn call(engine: *mut BtEngine, request: serde_json::Value) -> serde_json::Value {
    unsafe {
        let request = CString::new(request.to_string()).unwrap();
        let response = bt_call(engine, request.as_ptr());
        let parsed = serde_json::from_slice(CStr::from_ptr(response).to_bytes()).unwrap();
        bt_string_free(response);
        parsed
    }
}
fn bytes(text: &str) -> BuntrailBytes {
    BuntrailBytes {
        data: text.as_ptr(),
        length: text.len() as u64,
    }
}
#[test]
fn shared_json_native_state_snapshot_lifetime_and_boundary_validation() {
    unsafe {
        let engine = bt_engine_new();
        let opened = call(
            engine,
            serde_json::json!({"command":"document.open","text":"a🐇z"}),
        );
        let id = opened["value"]["id"].as_u64().unwrap();
        let mut snapshot = ptr::null_mut();
        assert_eq!(
            buntrail_snapshot_acquire_document(engine, id, &mut snapshot),
            BuntrailStatus::BUNTRAIL_STATUS_OK
        );
        assert_eq!(
            buntrail_snapshot_acquire_document(engine, id, ptr::null_mut()),
            BuntrailStatus::BUNTRAIL_STATUS_INVALID_ARGUMENT
        );
        let mut batch = ptr::null_mut();
        let mut anchor = BuntrailRange::default();
        let bad = BuntrailReplacement {
            start_utf16: 2,
            end_utf16: 3,
            text: bytes("x"),
        };
        assert_eq!(
            buntrail_document_transaction(
                engine,
                id,
                0,
                1,
                1,
                &bad,
                1,
                ptr::null(),
                0,
                &mut batch,
                &mut anchor
            ),
            BuntrailStatus::BUNTRAIL_STATUS_INVALID_ARGUMENT
        );
        assert!(batch.is_null());
        let replacement = BuntrailReplacement {
            start_utf16: 1,
            end_utf16: 3,
            text: bytes("兔"),
        };
        assert_eq!(
            buntrail_document_transaction(
                engine,
                id,
                0,
                1,
                1,
                &replacement,
                1,
                ptr::null(),
                0,
                &mut batch,
                &mut anchor
            ),
            BuntrailStatus::BUNTRAIL_STATUS_OK
        );
        assert_eq!(buntrail_batch_count(batch), 1);
        assert_eq!(
            call(
                engine,
                serde_json::json!({"command":"document.get","id":id})
            )["value"]["text"],
            "a兔z"
        );
        buntrail_batch_release(&mut batch);
        assert!(batch.is_null());
        let changed = call(
            engine,
            serde_json::json!({"command":"document.edit","id":id,"revision":1,"start":0,"end":1,"text":"A"}),
        );
        assert_eq!(changed["ok"], true);
        let mut current = ptr::null_mut();
        assert_eq!(
            buntrail_snapshot_acquire_document(engine, id, &mut current),
            BuntrailStatus::BUNTRAIL_STATUS_OK
        );
        assert_eq!(buntrail_snapshot_revision(current), 2);
        let mut retained = ptr::null_mut();
        assert_eq!(
            buntrail_snapshot_retain(snapshot, &mut retained),
            BuntrailStatus::BUNTRAIL_STATUS_OK
        );
        buntrail_snapshot_release(&mut snapshot);
        bt_engine_free(engine);
        let mut output = [0u8; 32];
        let n = buntrail_snapshot_read(retained, 0, output.as_mut_ptr(), 32);
        assert_eq!(&output[..n as usize], "a🐇z".as_bytes());
        buntrail_snapshot_release(&mut retained);
        buntrail_snapshot_release(&mut retained);
        buntrail_snapshot_release(&mut current);
    }
}
#[test]
fn find_plans_reject_foreign_engines_stale_and_closed_documents() {
    unsafe {
        let engine = bt_engine_new();
        let other = bt_engine_new();
        let open = serde_json::json!({"command":"document.open","text":"rabbit rabbit"});
        let id = call(engine, open.clone())["value"]["id"].as_u64().unwrap();
        call(other, open);
        let mut snapshot = ptr::null_mut();
        buntrail_snapshot_acquire_document(engine, id, &mut snapshot);
        let mut plan = ptr::null_mut();
        assert_eq!(
            buntrail_find_plan(
                snapshot,
                bytes("rabbit"),
                1,
                0,
                ptr::null(),
                bytes("🐇"),
                &mut plan
            ),
            BuntrailStatus::BUNTRAIL_STATUS_OK
        );
        let mut batch = ptr::null_mut();
        assert_eq!(
            buntrail_find_replace(other, id, 1, plan, -1, &mut batch),
            BuntrailStatus::BUNTRAIL_STATUS_STALE
        );
        call(
            engine,
            serde_json::json!({"command":"document.edit","id":id,"revision":0,"start":0,"end":0,"text":"!"}),
        );
        assert_eq!(
            buntrail_find_replace(engine, id, 1, plan, -1, &mut batch),
            BuntrailStatus::BUNTRAIL_STATUS_STALE
        );
        call(
            engine,
            serde_json::json!({"command":"document.close","id":id,"discard":true}),
        );
        assert_eq!(
            buntrail_find_replace(engine, id, 1, plan, -1, &mut batch),
            BuntrailStatus::BUNTRAIL_STATUS_CLOSED
        );
        buntrail_find_release(&mut plan);
        buntrail_find_release(&mut plan);
        buntrail_snapshot_release(&mut snapshot);
        bt_engine_free(engine);
        bt_engine_free(other);
    }
}

#[test]
fn native_outputs_utf8_and_large_delta_bytes_remain_validated() {
    unsafe {
        let engine = bt_engine_new();
        let opened = call(
            engine,
            serde_json::json!({"command":"document.open","text":""}),
        );
        let id = opened["value"]["id"].as_u64().unwrap();
        let mut snapshot = ptr::null_mut();
        assert_eq!(
            buntrail_snapshot_acquire_document(engine, id, &mut snapshot),
            BuntrailStatus::BUNTRAIL_STATUS_OK
        );
        let original = snapshot;
        assert_eq!(
            buntrail_snapshot_acquire_document(engine, id, &mut snapshot),
            BuntrailStatus::BUNTRAIL_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(snapshot, original);
        let mut batch = ptr::null_mut();
        let mut anchor = BuntrailRange::default();
        let invalid = [0xff];
        let replacement = BuntrailReplacement {
            start_utf16: 0,
            end_utf16: 0,
            text: BuntrailBytes {
                data: invalid.as_ptr(),
                length: 1,
            },
        };
        assert_eq!(
            buntrail_document_transaction(
                engine,
                id,
                0,
                1,
                1,
                &replacement,
                1,
                ptr::null(),
                0,
                &mut batch,
                &mut anchor
            ),
            BuntrailStatus::BUNTRAIL_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            buntrail_document_transaction(
                engine,
                id,
                0,
                1,
                1,
                ptr::null(),
                1,
                ptr::null(),
                0,
                &mut batch,
                &mut anchor
            ),
            BuntrailStatus::BUNTRAIL_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            buntrail_document_transaction(
                engine,
                id,
                0,
                1,
                1,
                ptr::null(),
                0,
                ptr::null(),
                0,
                &mut batch,
                ptr::null_mut()
            ),
            BuntrailStatus::BUNTRAIL_STATUS_INVALID_ARGUMENT
        );
        let text = "x".repeat(80_000);
        let replacement = BuntrailReplacement {
            start_utf16: 0,
            end_utf16: 0,
            text: bytes(&text),
        };
        assert_eq!(
            buntrail_document_transaction(
                engine,
                id,
                0,
                1,
                1,
                &replacement,
                1,
                ptr::null(),
                0,
                &mut batch,
                &mut anchor
            ),
            BuntrailStatus::BUNTRAIL_STATUS_OK
        );
        let mut info = BuntrailEventInfo::default();
        assert!(buntrail_batch_info(batch, 0, &mut info));
        assert_eq!(info.byte_count, 80_000);
        let mut copied = vec![0; 80_000];
        assert_eq!(
            buntrail_batch_copy(batch, 0, 0, copied.as_mut_ptr(), 80_000),
            80_000
        );
        assert!(copied.iter().all(|b| *b == b'x'));
        assert_eq!(buntrail_snapshot_read(snapshot, 0, ptr::null_mut(), 0), 0);
        let mut mapped = 99;
        assert_eq!(
            buntrail_snapshot_map_offset(snapshot, 0, 1, 1, &mut mapped),
            BuntrailStatus::BUNTRAIL_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(mapped, 99);
        buntrail_batch_release(&mut batch);
        buntrail_snapshot_release(&mut snapshot);
        assert!(snapshot.is_null());
        assert_eq!(buntrail_snapshot_length(snapshot), 0);
        bt_engine_free(engine);
    }
}
