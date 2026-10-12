/** @file
 * @brief Fixed-state 48 kHz DTMF detection descriptor ABI.
 */
#ifndef RPTADV_DTMF_H
#define RPTADV_DTMF_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define RPTADV_DTMF_ABI_VERSION 1U /**< Initial DTMF descriptor ABI. */

/** @brief Opaque state for one 48 kHz receive stream. */
struct rptadv_dtmf;

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
};

/** @brief Return the immutable descriptor; validate identity, size, and ABI. */
const struct rptadv_dtmf_descriptor *rptadv_dtmf_descriptor(void);

#ifdef __cplusplus
}
#endif

#endif
