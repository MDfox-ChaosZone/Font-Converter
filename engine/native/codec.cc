#include <woff2/encode.h>
#include <woff2/decode.h>
#include <woff2/output.h>
#include <cstddef>
#include <cstdint>

extern "C" size_t fc_capacity(const uint8_t* data, size_t length, int encode) noexcept {
  if (!data || !length) return 0;
  try {
    return encode ? woff2::MaxWOFF2CompressedSize(data, length)
                  : woff2::ComputeWOFF2FinalSize(data, length);
  } catch (...) { return 0; }
}

extern "C" int fc_convert(const uint8_t* data, size_t length, uint8_t* output,
                           size_t capacity, size_t* written, int encode) noexcept {
  if (!data || !length || !output || !capacity || !written) return 0;
  *written = 0;
  try {
    if (encode) {
      woff2::WOFF2Params params;
      params.brotli_quality = 11;
      params.allow_transforms = true;
      size_t size = capacity;
      if (!woff2::ConvertTTFToWOFF2(data, length, output, &size, params) || size > capacity) return 0;
      *written = size;
    } else {
      woff2::WOFF2MemoryOut writer(output, capacity);
      if (!woff2::ConvertWOFF2ToTTF(data, length, &writer) || writer.Size() > capacity) return 0;
      *written = writer.Size();
    }
    return *written ? 1 : 0;
  } catch (...) { return 0; }
}
