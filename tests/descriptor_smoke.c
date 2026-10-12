#include "rptadvdtmf/rptadvdtmf.h"

#include <assert.h>
#include <stddef.h>
#include <string.h>

static void capture(void *context, uint8_t digit) {
  uint8_t *result = context;
  *result = digit;
}

int main(void) {
  const struct rptadv_dtmf_descriptor *api = rptadv_dtmf_descriptor();
  assert(api == rptadv_dtmf_descriptor());
  assert(api->struct_size == sizeof(*api));
  assert(api->abi_version == RPTADV_DTMF_ABI_VERSION);
  assert(strcmp(api->capability_name, "rptadv.dtmf") == 0);
  assert(api->create && api->destroy && api->set_muting && api->suppressing &&
         api->process);

  assert(api->create(1, NULL) == -1);
  struct rptadv_dtmf *detector = NULL;
  assert(api->create(2, &detector) == 0);
  assert(detector);
  float audio[1] = {0.25f};
  uint8_t digit = 0;
  api->process(detector, 2, audio, 1, capture, &digit);
  assert(audio[0] == 0.25f && digit == 0);
  api->process(detector, 0, NULL, 0, capture, &digit);
  assert(api->suppressing(detector) == 0);
  api->set_muting(detector, 0);
  api->destroy(detector);
  api->destroy(NULL);
  return 0;
}
