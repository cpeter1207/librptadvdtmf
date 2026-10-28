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
  assert(api->generator_create && api->generator_render &&
         api->generator_destroy);
  assert(api->create_at_rate && api->process_spans);
  assert(api->create_at_rate(1, 8000, &detector) == 0);
  api->process_spans(detector, 0, NULL, 0, NULL, NULL, NULL, NULL);
  api->destroy(detector);
  assert(api->create_at_rate(1, 44100, &detector) == -1 && detector == NULL);
  struct rptadv_dtmf_generator *generator = NULL;
  assert(api->generator_create('5', 1, 1, -12, &generator) == 0);
  float output[97] = {0};
  size_t written = 0;
  assert(api->generator_render(generator, output, 97, &written) == 0);
  assert(written == 96 && output[1] > 0);
  for (size_t i = 48; i < 96; ++i) {
    assert(output[i] == 0);
  }
  api->generator_destroy(generator);
  api->generator_destroy(NULL);
  return 0;
}
