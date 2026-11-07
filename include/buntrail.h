#ifndef BUNTRAIL_H
#define BUNTRAIL_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef enum BuntrailStatus {
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
} BuntrailStatus;

typedef struct BtEngine BtEngine;

typedef struct BuntrailBatch BuntrailBatch;

typedef struct BuntrailFindPlan BuntrailFindPlan;

typedef struct BuntrailSnapshot BuntrailSnapshot;

typedef struct BuntrailBytes {
  const uint8_t *data;
  uint64_t length;
} BuntrailBytes;

typedef struct BuntrailRange {
  uint64_t revision;
  uint64_t start_utf16;
  uint64_t end_utf16;
} BuntrailRange;

typedef struct BuntrailReplacement {
  uint64_t start_utf16;
  uint64_t end_utf16;
  struct BuntrailBytes text;
} BuntrailReplacement;

typedef struct BuntrailEventInfo {
  uint32_t size;
  uint32_t kind;
  uint64_t revision;
  uint64_t digest;
  uint64_t start_utf16;
  uint64_t length_utf16;
  uint64_t byte_count;
  uint64_t span_count;
} BuntrailEventInfo;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

uint32_t bt_abi_version(void);

struct BtEngine *bt_engine_new(void);

/**
 * # Safety
 * `engine` must be null or an engine returned by bt_engine_new, freed exactly
 * once, after every call using it has returned.
 */
void bt_engine_free(struct BtEngine *engine);

/**
 * Calls are thread-safe; UI code serializes composition/edit transactions.
 * No GUI callback is invoked. Perform service calls on a worker thread.
 * # Safety
 * `engine` must be live. `request` must be a readable NUL-terminated string
 * throughout this call. Neither pointer may be freed concurrently.
 */
char *bt_call(struct BtEngine *engine, const char *request);

/**
 * # Safety
 * `value` must be null or a string returned by bt_call, freed exactly once.
 */
void bt_string_free(char *value);

/**
 * # Safety
 * Live existing engine and a writable null output slot; snapshot owns immutable text.
 */
enum BuntrailStatus buntrail_snapshot_acquire_document(struct BtEngine *engine,
                                                       uint64_t document,
                                                       struct BuntrailSnapshot **out);

/**
 * # Safety
 * Live immutable snapshot and writable null output slot. May run on any thread.
 */
enum BuntrailStatus buntrail_snapshot_retain(const struct BuntrailSnapshot *snapshot,
                                             struct BuntrailSnapshot **out);

/**
 * Reads bounded UTF-8 bytes from a retained snapshot. Returns zero past end; offsets are byte units.
 * # Safety
 * Live snapshot; output writable for capacity bytes. No concurrent release.
 */
uint64_t buntrail_snapshot_read(const struct BuntrailSnapshot *snapshot,
                                uint64_t offset,
                                uint8_t *out,
                                uint64_t capacity);

/**
 * # Safety
 * Live snapshot, or null (returns zero).
 */
uint64_t buntrail_snapshot_revision(const struct BuntrailSnapshot *snapshot);

/**
 * # Safety
 * Unique writable slot; no concurrent read/retain. Null contents are idempotent.
 */
void buntrail_snapshot_release(struct BuntrailSnapshot **slot);

/**
 * # Safety
 * Live immutable snapshot, or null (returns zero).
 */
uint64_t buntrail_snapshot_length(const struct BuntrailSnapshot *snapshot);

/**
 * Maps a boundary in an exact retained snapshot. unit 1: UTF-16 to UTF-8; 2: UTF-8 to UTF-16.
 * # Safety
 * Live immutable snapshot and writable output; no concurrent release.
 */
enum BuntrailStatus buntrail_snapshot_map_offset(const struct BuntrailSnapshot *snapshot,
                                                 uint64_t revision,
                                                 uint32_t unit,
                                                 uint64_t offset,
                                                 uint64_t *out);

/**
 * # Safety
 * Live snapshot, UTF-8 options/replacement, optional readable revision-tagged scope and null output slot.
 */
enum BuntrailStatus buntrail_find_plan(const struct BuntrailSnapshot *snapshot,
                                       struct BuntrailBytes expression,
                                       uint32_t mode,
                                       uint32_t flags,
                                       const struct BuntrailRange *scope,
                                       struct BuntrailBytes replacement,
                                       struct BuntrailFindPlan **out);

/**
 * # Safety
 * Live plan and writable outputs. Error text is borrowed until plan release.
 */
bool buntrail_find_info(const struct BuntrailFindPlan *plan,
                        uint64_t *count,
                        bool *limited,
                        struct BuntrailBytes *error);

/**
 * # Safety
 * Live plan and writable range output.
 */
bool buntrail_find_range(const struct BuntrailFindPlan *plan,
                         uint64_t index,
                         struct BuntrailRange *out);

/**
 * # Safety
 * Unique writable plan slot; no concurrent access. Null contents are idempotent.
 */
void buntrail_find_release(struct BuntrailFindPlan **slot);

/**
 * # Safety
 * Live engine; replacements/text/selection readable; writable null batch slot and anchor.
 */
enum BuntrailStatus buntrail_document_transaction(struct BtEngine *engine,
                                                  uint64_t document,
                                                  uint64_t revision,
                                                  uint64_t view,
                                                  uint32_t intent,
                                                  const struct BuntrailReplacement *replacements,
                                                  uint64_t count,
                                                  const struct BuntrailRange *selection,
                                                  uint32_t typing,
                                                  struct BuntrailBatch **out,
                                                  struct BuntrailRange *selection_out);

/**
 * # Safety
 * Live immutable batch, or null (zero); no concurrent release.
 */
uint64_t buntrail_batch_count(const struct BuntrailBatch *batch);

/**
 * # Safety
 * Live batch, writable output; no concurrent release.
 */
bool buntrail_batch_info(const struct BuntrailBatch *batch,
                         uint64_t index,
                         struct BuntrailEventInfo *out);

/**
 * # Safety
 * Live batch, output writable for capacity bytes; no concurrent release.
 */
uint64_t buntrail_batch_copy(const struct BuntrailBatch *batch,
                             uint64_t index,
                             uint64_t offset,
                             uint8_t *out,
                             uint64_t capacity);

/**
 * # Safety
 * Unique writable owned batch slot; no concurrent accesses. Null/empty is idempotent.
 */
void buntrail_batch_release(struct BuntrailBatch **slot);

/**
 * # Safety
 * Live engine/plan; null batch output slot. Negative index replaces all.
 */
enum BuntrailStatus buntrail_find_replace(struct BtEngine *engine,
                                          uint64_t document,
                                          uint64_t view,
                                          const struct BuntrailFindPlan *plan,
                                          int64_t index,
                                          struct BuntrailBatch **out);

/**
 * # Safety
 * Live engine and valid UTF-8 language bytes for call; empty resets association.
 */
enum BuntrailStatus buntrail_document_set_language(struct BtEngine *engine,
                                                   uint64_t document,
                                                   struct BuntrailBytes language);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* BUNTRAIL_H */
