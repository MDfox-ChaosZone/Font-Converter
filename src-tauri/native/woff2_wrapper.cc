#include <woff2/encode.h>
#include <woff2/decode.h>
#include <woff2/output.h>

#include <cstddef>
#include <cstdint>

extern "C" size_t ttf2woff2_google_max_compressed_size(
    const uint8_t* input,
    size_t input_length) noexcept {
  if (input == nullptr || input_length == 0) {
    return 0;
  }

  try {
    return woff2::MaxWOFF2CompressedSize(input, input_length);
  } catch (...) {
    return 0;
  }
}

extern "C" int ttf2woff2_google_convert(
    const uint8_t* input,
    size_t input_length,
    uint8_t* output,
    size_t* output_length,
    int brotli_quality,
    int allow_transforms) noexcept {
  if (input == nullptr || input_length == 0 || output == nullptr ||
      output_length == nullptr) {
    return false;
  }

  try {
    woff2::WOFF2Params params;
    params.brotli_quality = brotli_quality;
    params.allow_transforms = allow_transforms != 0;
    return woff2::ConvertTTFToWOFF2(
               input, input_length, output, output_length, params)
               ? 1
               : 0;
  } catch (...) {
    return 0;
  }
}

extern "C" size_t ttf2woff2_google_decompressed_size(
    const uint8_t* input,
    size_t input_length) noexcept {
  if (input == nullptr || input_length == 0) {
    return 0;
  }

  try {
    return woff2::ComputeWOFF2FinalSize(input, input_length);
  } catch (...) {
    return 0;
  }
}

extern "C" int ttf2woff2_google_decompress(
    const uint8_t* input,
    size_t input_length,
    uint8_t* output,
    size_t output_capacity,
    size_t* output_length) noexcept {
  if (input == nullptr || input_length == 0 || output == nullptr ||
      output_capacity == 0 || output_length == nullptr) {
    return 0;
  }

  try {
    woff2::WOFF2MemoryOut writer(output, output_capacity);
    if (!woff2::ConvertWOFF2ToTTF(input, input_length, &writer)) {
      return 0;
    }
    *output_length = writer.Size();
    return 1;
  } catch (...) {
    return 0;
  }
}
