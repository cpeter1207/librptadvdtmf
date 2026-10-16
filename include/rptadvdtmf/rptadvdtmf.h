/** @file
 * @brief 48 kHz DTMF detection and finite PCM generation descriptor ABI.
 */
#ifndef RPTADV_DTMF_H
#define RPTADV_DTMF_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

#define RPTADV_DTMF_ABI_VERSION 1U /**< Initial DTMF descriptor ABI. */

/** @brief Opaque state for one 48 kHz receive stream. */
struct rptadv_dtmf;
/** @brief Owned finite tone and following silence; create and destroy off audio. */
struct rptadv_dtmf_generator;

/** @brief Completed digit callback; context is borrowed only during process. */
typedef void (*rptadv_dtmf_emit_fn)(void *context, uint8_t ascii_digit);

/** @brief Immutable function table for the process lifetime. */
struct rptadv_dtmf_descriptor {
  uint32_t struct_size; /**< Size of this descriptor. */
  uint32_t abi_version; /**< Must equal RPTADV_DTMF_ABI_VERSION. */
  const char *capability_name; /**< Stable `rptadv.dtmf` identity. */

  /** @brief Allocate detector state before audio starts.
   * @param muting Zero disables muting; nonzero enables it.
   * @param out Receives the owned opaque handle. Null returns -1.
   * @return Zero on success, -1 for null out.
   */
  int32_t (*create)(uint32_t muting, struct rptadv_dtmf **out);
  /** @brief Release state after its audio callback has stopped; null is accepted. */
  void (*destroy)(struct rptadv_dtmf *handle);
  /** @brief Change qualified-tone muting without resetting detection;
   * zero disables and nonzero enables it.
   */
  void (*set_muting)(struct rptadv_dtmf *handle, uint32_t enabled);
  /** @brief Return 1 when a qualified tone is muted, otherwise 0. */
  uint32_t (*suppressing)(const struct rptadv_dtmf *handle);
  /** @brief Consume and optionally mute one arbitrary 48 kHz mono F32 partition.
   * @param handle Exclusive detector state for this call.
   * @param receiving Zero completes a qualified digit and resets state, even
   *                  when count is zero and audio is null; nonzero receives.
   * @param audio Writable for count samples; null is accepted only when count is zero.
   * @param count Number of samples in this partition.
   * @param emit Optional borrowed callback for completed ASCII `0`-`9`, `A`-`D`,
   *             `*`, or `#` events, in order. It must not unwind, allocate, lock,
   *             block, or perform I/O on the audio worker.
   * @param context Borrowed pointer valid only until process returns.
   * @note Invalid handle or nonzero count with null audio is ignored.
   */
  void (*process)(struct rptadv_dtmf *handle, uint32_t receiving, float *audio,
                  uint32_t count, rptadv_dtmf_emit_fn emit, void *context);
  /** @brief Allocate a finite 48 kHz mono F32 digit followed by silence.
   * @param digit ASCII 0-9, A-D, star, or hash.
   * @param duration_ms Tone duration, 1 through 1000 milliseconds.
   * @param gap_ms Following silence, 1 through 1000 milliseconds.
   * @param level_dbfs Combined peak, -60 through 0 dBFS; equal component levels.
   * @param out Receives the owned handle; cleared on failure.
   * @return Zero on success, -1 for invalid arguments or provider failure.
   * @note ABI 1 extension: check struct_size before reading this tail.
   */
  int32_t (*generator_create)(uint8_t digit, uint32_t duration_ms,
                             uint32_t gap_ms, int32_t level_dbfs,
                             struct rptadv_dtmf_generator **out);
  /** @brief Render up to capacity samples without allocation or blocking.
   * @param handle Exclusive live generator.
   * @param out Writable span; null permitted only for zero capacity.
   * @param capacity Number of writable samples.
   * @param written Receives sample count, including the trailing silence.
   * @return Zero on success, -1 on invalid arguments or provider failure.
   * @note Zero written means finished when capacity is nonzero. Unwritten
   * output is untouched; oscillator phase is preserved between calls.
   */
  int32_t (*generator_render)(struct rptadv_dtmf_generator *handle, float *out,
                             size_t capacity, size_t *written);
  /** @brief Release generator after its last render; null is accepted. */
  void (*generator_destroy)(struct rptadv_dtmf_generator *handle);
};

/** @brief Return the immutable descriptor; validate identity, size, and ABI. */
const struct rptadv_dtmf_descriptor *rptadv_dtmf_descriptor(void);

#ifdef __cplusplus
}
#endif

#endif
