//! Editor ABI algorithms using the BtEngine document registry and IDs.
use super::*;
use buntrail_core::{Delta, Snapshot, Text, contracts::*};
use std::sync::Arc;
#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum BuntrailStatus {
    BUNTRAIL_STATUS_OK = 0,
    BUNTRAIL_STATUS_INVALID_ARGUMENT = 1,
    BUNTRAIL_STATUS_ABI_MISMATCH = 2,
    BUNTRAIL_STATUS_WRONG_THREAD = 3,
    BUNTRAIL_STATUS_NOT_READY = 4,
    BUNTRAIL_STATUS_STALE = 5,
    BUNTRAIL_STATUS_COMPOSITION = 6,
    BUNTRAIL_STATUS_IO = 7,
    BUNTRAIL_STATUS_POISONED = 8,
    BUNTRAIL_STATUS_CLOSED = 9,
    BUNTRAIL_STATUS_UNAVAILABLE = 10,
    BUNTRAIL_STATUS_BUSY = 11,
}
use BuntrailStatus::*;
#[repr(C)]
#[derive(Default)]
pub struct BuntrailBytes {
    pub data: *const u8,
    pub length: u64,
}
#[repr(C)]
#[derive(Default)]
pub struct BuntrailRange {
    pub revision: u64,
    pub start_utf16: u64,
    pub end_utf16: u64,
}
#[repr(C)]
pub struct BuntrailReplacement {
    pub start_utf16: u64,
    pub end_utf16: u64,
    pub text: BuntrailBytes,
}
#[repr(C)]
#[derive(Default)]
pub struct BuntrailEventInfo {
    pub size: u32,
    pub kind: u32,
    pub revision: u64,
    pub digest: u64,
    pub start_utf16: u64,
    pub length_utf16: u64,
    pub byte_count: u64,
    pub span_count: u64,
}
pub struct BuntrailSnapshot {
    document: Option<u64>,
    snapshot: Snapshot,
    owner: Arc<()>,
}
pub struct BuntrailBatch {
    deltas: Vec<Delta>,
}
pub struct BuntrailFindPlan {
    document: Option<u64>,
    source: Arc<Text>,
    owner: Arc<()>,
    result: Result<buntrail_core::search::find::FindPlan, CoreError>,
}
fn core_status(error: CoreError) -> BuntrailStatus {
    match error.code {
        ErrorCode::StaleRevision => BUNTRAIL_STATUS_STALE,
        ErrorCode::Composition => BUNTRAIL_STATUS_COMPOSITION,
        ErrorCode::Unavailable => BUNTRAIL_STATUS_UNAVAILABLE,
        ErrorCode::Busy => BUNTRAIL_STATUS_BUSY,
        ErrorCode::Closed => BUNTRAIL_STATUS_CLOSED,
        ErrorCode::Io => BUNTRAIL_STATUS_IO,
        _ => BUNTRAIL_STATUS_INVALID_ARGUMENT,
    }
}
fn engine_status(error: buntrail_engine::Failure) -> BuntrailStatus {
    match error.code {
        "stale_revision" => BUNTRAIL_STATUS_STALE,
        "not_found" => BUNTRAIL_STATUS_CLOSED,
        "composition" => BUNTRAIL_STATUS_COMPOSITION,
        "engine_failed" => BUNTRAIL_STATUS_POISONED,
        _ => BUNTRAIL_STATUS_INVALID_ARGUMENT,
    }
}
unsafe fn text<'a>(data: *const u8, length: u64) -> Result<&'a str, BuntrailStatus> {
    if length > isize::MAX as u64 || (length > 0 && data.is_null()) {
        return Err(BUNTRAIL_STATUS_INVALID_ARGUMENT);
    }
    if length == 0 {
        return Ok("");
    }
    std::str::from_utf8(unsafe { std::slice::from_raw_parts(data, length as usize) })
        .map_err(|_| BUNTRAIL_STATUS_INVALID_ARGUMENT)
}
unsafe fn run(
    engine: *mut BtEngine,
    f: impl FnOnce(&BtEngine) -> Result<(), BuntrailStatus>,
) -> BuntrailStatus {
    let Some(engine) = (unsafe { engine.as_ref() }) else {
        return BUNTRAIL_STATUS_INVALID_ARGUMENT;
    };
    if engine.failed.load(Ordering::Acquire) {
        return BUNTRAIL_STATUS_POISONED;
    }
    match catch_unwind(AssertUnwindSafe(|| f(engine))) {
        Ok(Ok(())) => BUNTRAIL_STATUS_OK,
        Ok(Err(status)) => status,
        Err(_) => {
            engine.failed.store(true, Ordering::Release);
            BUNTRAIL_STATUS_POISONED
        }
    }
}
unsafe fn output(out: *mut *mut BuntrailBatch, deltas: Vec<Delta>) -> Result<(), BuntrailStatus> {
    if out.is_null() || !unsafe { (*out).is_null() } {
        return Err(BUNTRAIL_STATUS_INVALID_ARGUMENT);
    }
    unsafe {
        *out = Box::into_raw(Box::new(BuntrailBatch { deltas }));
    }
    Ok(())
}
/// # Safety
/// Live existing engine and a writable null output slot; snapshot owns immutable text.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buntrail_snapshot_acquire_document(
    engine: *mut BtEngine,
    document: u64,
    out: *mut *mut BuntrailSnapshot,
) -> BuntrailStatus {
    unsafe {
        run(engine, |e| {
            if out.is_null() || !(*out).is_null() {
                return Err(BUNTRAIL_STATUS_INVALID_ARGUMENT);
            }
            let snapshot = e.engine.snapshot(document).map_err(engine_status)?;
            *out = Box::into_raw(Box::new(BuntrailSnapshot {
                document: Some(document),
                snapshot,
                owner: e.identity.clone(),
            }));
            Ok(())
        })
    }
}
/// # Safety
/// Live immutable snapshot and writable null output slot. May run on any thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buntrail_snapshot_retain(
    snapshot: *const BuntrailSnapshot,
    out: *mut *mut BuntrailSnapshot,
) -> BuntrailStatus {
    if snapshot.is_null() || out.is_null() || !unsafe { (*out).is_null() } {
        return BUNTRAIL_STATUS_INVALID_ARGUMENT;
    }
    unsafe {
        *out = Box::into_raw(Box::new(BuntrailSnapshot {
            snapshot: (*snapshot).snapshot.clone(),
            document: (*snapshot).document,
            owner: (*snapshot).owner.clone(),
        }));
    }
    BUNTRAIL_STATUS_OK
}

/// Reads bounded UTF-8 bytes from a retained snapshot. Returns zero past end; offsets are byte units.
/// # Safety
/// Live snapshot; output writable for capacity bytes. No concurrent release.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buntrail_snapshot_read(
    snapshot: *const BuntrailSnapshot,
    offset: u64,
    out: *mut u8,
    capacity: u64,
) -> u64 {
    let Some(s) = (unsafe { snapshot.as_ref() }) else {
        return 0;
    };
    if out.is_null() || offset > usize::MAX as u64 {
        return 0;
    }
    let capacity = capacity.min(65536) as usize;
    s.snapshot.text.read(offset as usize, unsafe {
        std::slice::from_raw_parts_mut(out, capacity)
    }) as u64
}

/// # Safety
/// Live snapshot, or null (returns zero).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buntrail_snapshot_revision(snapshot: *const BuntrailSnapshot) -> u64 {
    unsafe { snapshot.as_ref() }.map_or(0, |s| s.snapshot.revision)
}

/// # Safety
/// Unique writable slot; no concurrent read/retain. Null contents are idempotent.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buntrail_snapshot_release(slot: *mut *mut BuntrailSnapshot) {
    if !slot.is_null() && !unsafe { (*slot).is_null() } {
        unsafe {
            drop(Box::from_raw(*slot));
            *slot = std::ptr::null_mut();
        }
    }
}

/// # Safety
/// Live immutable snapshot, or null (returns zero).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buntrail_snapshot_length(snapshot: *const BuntrailSnapshot) -> u64 {
    unsafe { snapshot.as_ref() }.map_or(0, |s| s.snapshot.text.len() as u64)
}

/// Maps a boundary in an exact retained snapshot. unit 1: UTF-16 to UTF-8; 2: UTF-8 to UTF-16.
/// # Safety
/// Live immutable snapshot and writable output; no concurrent release.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buntrail_snapshot_map_offset(
    snapshot: *const BuntrailSnapshot,
    revision: u64,
    unit: u32,
    offset: u64,
    out: *mut u64,
) -> BuntrailStatus {
    let (Some(snapshot), Some(out)) = (unsafe { snapshot.as_ref() }, unsafe { out.as_mut() })
    else {
        return BUNTRAIL_STATUS_INVALID_ARGUMENT;
    };
    if revision != snapshot.snapshot.revision {
        return BUNTRAIL_STATUS_STALE;
    }
    let Ok(offset) = usize::try_from(offset) else {
        return BUNTRAIL_STATUS_INVALID_ARGUMENT;
    };
    let value = match unit {
        1 => snapshot.snapshot.text.utf16_to_byte(offset),
        2 => snapshot.snapshot.text.byte_to_utf16(offset),
        _ => return BUNTRAIL_STATUS_INVALID_ARGUMENT,
    };
    match value {
        Ok(value) => {
            *out = value as u64;
            BUNTRAIL_STATUS_OK
        }
        Err(_) => BUNTRAIL_STATUS_INVALID_ARGUMENT,
    }
}

/// # Safety
/// Live snapshot, UTF-8 options/replacement, optional readable revision-tagged scope and null output slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buntrail_find_plan(
    snapshot: *const BuntrailSnapshot,
    expression: BuntrailBytes,
    mode: u32,
    flags: u32,
    scope: *const BuntrailRange,
    replacement: BuntrailBytes,
    out: *mut *mut BuntrailFindPlan,
) -> BuntrailStatus {
    unsafe {
        let Some(snapshot) = snapshot.as_ref() else {
            return BUNTRAIL_STATUS_INVALID_ARGUMENT;
        };
        if out.is_null()
            || !(*out).is_null()
            || !(1..=2).contains(&mode)
            || flags & !3 != 0
            || expression.length > 65536
            || replacement.length > 64 * 1024 * 1024
        {
            return BUNTRAIL_STATUS_INVALID_ARGUMENT;
        }
        let result = (|| {
            use buntrail_core::contracts::*;
            let expression = text(expression.data, expression.length).map_err(|_| {
                CoreError::new(ErrorCode::InvalidArgument, "Invalid find expression")
            })?;
            let replacement = text(replacement.data, replacement.length)
                .map_err(|_| CoreError::new(ErrorCode::InvalidArgument, "Invalid replacement"))?;
            buntrail_core::search::find::plan(
                &snapshot.snapshot.text,
                BufferRevision(snapshot.snapshot.revision),
                &buntrail_core::search::find::FindOptions {
                    expression: expression.into(),
                    mode: if mode == 1 {
                        SearchMode::Literal
                    } else {
                        SearchMode::Regex
                    },
                    case_sensitive: flags & 1 != 0,
                    whole_word: flags & 2 != 0,
                    scope: scope.as_ref().map(|r| BufferRange {
                        revision: BufferRevision(r.revision),
                        start: r.start_utf16,
                        end: r.end_utf16,
                    }),
                },
                replacement,
            )
        })();
        *out = Box::into_raw(Box::new(BuntrailFindPlan {
            result,
            owner: snapshot.owner.clone(),
            document: snapshot.document,
            source: snapshot.snapshot.text.clone(),
        }));
        BUNTRAIL_STATUS_OK
    }
}

/// # Safety
/// Live plan and writable outputs. Error text is borrowed until plan release.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buntrail_find_info(
    plan: *const BuntrailFindPlan,
    count: *mut u64,
    limited: *mut bool,
    error: *mut BuntrailBytes,
) -> bool {
    unsafe {
        let Some(p) = plan.as_ref() else {
            return false;
        };
        if count.is_null() || limited.is_null() || error.is_null() {
            return false;
        }
        *count = 0;
        *limited = false;
        *error = BuntrailBytes::default();
        match &p.result {
            Ok(p) => {
                *count = p.matches.len() as u64;
                *limited = p.limited;
            }
            Err(e) => {
                *error = BuntrailBytes {
                    data: e.message.as_ptr(),
                    length: e.message.len() as u64,
                }
            }
        }
        true
    }
}

/// # Safety
/// Live plan and writable range output.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buntrail_find_range(
    plan: *const BuntrailFindPlan,
    index: u64,
    out: *mut BuntrailRange,
) -> bool {
    unsafe {
        let Some(r) = plan
            .as_ref()
            .and_then(|p| p.result.as_ref().ok())
            .and_then(|p| p.matches.get(index as usize))
        else {
            return false;
        };
        if out.is_null() {
            return false;
        }
        *out = BuntrailRange {
            revision: r.revision.0,
            start_utf16: r.start,
            end_utf16: r.end,
        };
        true
    }
}

/// # Safety
/// Unique writable plan slot; no concurrent access. Null contents are idempotent.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buntrail_find_release(slot: *mut *mut BuntrailFindPlan) {
    unsafe {
        if !slot.is_null() && !(*slot).is_null() {
            drop(Box::from_raw(*slot));
            *slot = std::ptr::null_mut();
        }
    }
}
/// # Safety
/// Live engine; replacements/text/selection readable; writable null batch slot and anchor.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buntrail_document_transaction(
    engine: *mut BtEngine,
    document: u64,
    revision: u64,
    view: u64,
    intent: u32,
    replacements: *const BuntrailReplacement,
    count: u64,
    selection: *const BuntrailRange,
    typing: u32,
    out: *mut *mut BuntrailBatch,
    selection_out: *mut BuntrailRange,
) -> BuntrailStatus {
    unsafe {
        run(engine, |e| {
            if out.is_null()
                || !(*out).is_null()
                || selection_out.is_null()
                || count > 1024
                || (count > 0 && replacements.is_null())
                || typing > 2
            {
                return Err(BUNTRAIL_STATUS_INVALID_ARGUMENT);
            }
            let intent = match intent {
                1 => EditIntent::Replace,
                2 => EditIntent::Undo,
                3 => EditIntent::Redo,
                4 => EditIntent::BeginComposition,
                5 => EditIntent::UpdateComposition,
                6 => EditIntent::CommitComposition,
                7 => EditIntent::CancelComposition,
                _ => return Err(BUNTRAIL_STATUS_INVALID_ARGUMENT),
            };
            let mut changes = vec![];
            if count > 0 {
                for r in std::slice::from_raw_parts(replacements, count as usize) {
                    if r.text.length > 64 * 1024 * 1024 {
                        return Err(BUNTRAIL_STATUS_INVALID_ARGUMENT);
                    }
                    changes.push(Replacement {
                        range: BufferRange {
                            revision: BufferRevision(revision),
                            start: r.start_utf16,
                            end: r.end_utf16,
                        },
                        text: text(r.text.data, r.text.length)?.into(),
                    });
                }
            }
            let selection = selection.as_ref().map(|s| BufferRange {
                revision: BufferRevision(s.revision),
                start: s.start_utf16,
                end: s.end_utf16,
            });
            let result = e
                .engine
                .transaction(
                    document,
                    EditTransaction {
                        expected_revision: BufferRevision(revision),
                        source_view: ViewID(view),
                        replacements: changes,
                        intent,
                    },
                    selection,
                    typing,
                )
                .map_err(engine_status)?;
            let anchor = result.selection_anchors.last();
            *selection_out = BuntrailRange {
                revision: result.revision.0,
                start_utf16: anchor.map_or(0, |r| r.start),
                end_utf16: anchor.map_or(0, |r| r.end),
            };
            output(out, result.deltas)
        })
    }
}
/// # Safety
/// Live immutable batch, or null (zero); no concurrent release.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buntrail_batch_count(batch: *const BuntrailBatch) -> u64 {
    unsafe { batch.as_ref() }.map_or(0, |b| b.deltas.len() as u64)
}
/// # Safety
/// Live batch, writable output; no concurrent release.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buntrail_batch_info(
    batch: *const BuntrailBatch,
    index: u64,
    out: *mut BuntrailEventInfo,
) -> bool {
    let Some(delta) = unsafe { batch.as_ref() }.and_then(|b| b.deltas.get(index as usize)) else {
        return false;
    };
    let Some(out) = (unsafe { out.as_mut() }) else {
        return false;
    };
    *out = BuntrailEventInfo {
        size: std::mem::size_of::<BuntrailEventInfo>() as u32,
        kind: 1,
        revision: delta.revision,
        digest: buntrail_core::digest(delta.text.as_bytes()),
        start_utf16: delta.start_utf16 as u64,
        length_utf16: delta.removed_utf16 as u64,
        byte_count: delta.text.len() as u64,
        span_count: 0,
    };
    true
}
/// # Safety
/// Live batch, output writable for capacity bytes; no concurrent release.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buntrail_batch_copy(
    batch: *const BuntrailBatch,
    index: u64,
    offset: u64,
    out: *mut u8,
    capacity: u64,
) -> u64 {
    let Some(delta) = unsafe { batch.as_ref() }.and_then(|b| b.deltas.get(index as usize)) else {
        return 0;
    };
    if out.is_null() {
        return 0;
    }
    let Some(bytes) = usize::try_from(offset)
        .ok()
        .and_then(|offset| delta.text.as_bytes().get(offset..))
    else {
        return 0;
    };
    let count = bytes
        .len()
        .min(usize::try_from(capacity).unwrap_or(usize::MAX));
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, count);
    }
    count as u64
}
/// # Safety
/// Unique writable owned batch slot; no concurrent accesses. Null/empty is idempotent.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buntrail_batch_release(slot: *mut *mut BuntrailBatch) {
    if !slot.is_null() && !unsafe { (*slot).is_null() } {
        unsafe {
            drop(Box::from_raw(*slot));
            *slot = std::ptr::null_mut();
        }
    }
}
/// # Safety
/// Live engine/plan; null batch output slot. Negative index replaces all.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buntrail_find_replace(
    engine: *mut BtEngine,
    document: u64,
    view: u64,
    plan: *const BuntrailFindPlan,
    index: i64,
    out: *mut *mut BuntrailBatch,
) -> BuntrailStatus {
    unsafe {
        run(engine, |e| {
            if out.is_null() || !(*out).is_null() {
                return Err(BUNTRAIL_STATUS_INVALID_ARGUMENT);
            }
            let plan = plan.as_ref().ok_or(BUNTRAIL_STATUS_INVALID_ARGUMENT)?;
            let snapshot = e.engine.snapshot(document).map_err(engine_status)?;
            if plan.document != Some(document)
                || !Arc::ptr_eq(&plan.owner, &e.identity)
                || !Arc::ptr_eq(&plan.source, &snapshot.text)
            {
                return Err(BUNTRAIL_STATUS_STALE);
            }
            let p = plan.result.as_ref().map_err(|e| core_status(e.clone()))?;
            let replacements = if index < 0 {
                if p.limited {
                    return Err(BUNTRAIL_STATUS_UNAVAILABLE);
                }
                p.replacements.clone()
            } else {
                vec![
                    p.replacements
                        .get(index as usize)
                        .ok_or(BUNTRAIL_STATUS_INVALID_ARGUMENT)?
                        .clone(),
                ]
            };
            let result = e
                .engine
                .transaction(
                    document,
                    EditTransaction {
                        expected_revision: p.revision,
                        source_view: ViewID(view),
                        replacements,
                        intent: EditIntent::Replace,
                    },
                    None,
                    2,
                )
                .map_err(engine_status)?;
            output(out, result.deltas)
        })
    }
}
/// # Safety
/// Live engine and valid UTF-8 language bytes for call; empty resets association.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buntrail_document_set_language(
    engine: *mut BtEngine,
    document: u64,
    language: BuntrailBytes,
) -> BuntrailStatus {
    unsafe {
        run(engine, |e| {
            if language.length > 128 {
                return Err(BUNTRAIL_STATUS_INVALID_ARGUMENT);
            }
            let language = text(language.data, language.length)?;
            e.engine
                .set_language(
                    document,
                    (!language.is_empty()).then(|| language.to_owned()),
                )
                .map_err(engine_status)
        })
    }
}
