/* params: passes, fast_bytes -- AdvanceCOMP's compress_deflate_7z() (7z/7zdeflate.cc) minus the
 * zlib wrapper, which is irrelevant for raw DEFLATE. The output buffer is oversized instead of
 * being truncated by the caller's estimate. */
#include "fz.h"
#include "7z/DeflateEncoder.h"
#include "7z/IInOutStreams.h"
#include <vector>

extern "C" int FZ_ENTRY(run)(FZ_ARGS) {
    if (nparams != 2 || in_len > 0x7fffffff) return FZ_EPARAM;
    try {
        NDeflate::NEncoder::CCoder cc;
        if (cc.SetEncoderNumPasses((UINT32)params[0]) != S_OK) return FZ_EPARAM;
        if (cc.SetEncoderNumFastBytes((UINT32)params[1]) != S_OK) return FZ_EPARAM;
        /* worst case: stored blocks of 64 KiB-ish plus slack; Huffman blocks never exceed it */
        size_t cap = in_len + in_len / 100 + (1 << 16);
        std::vector<char> buf(cap);
        ISequentialInStream is(reinterpret_cast<const char *>(in), (INT)in_len);
        ISequentialOutStream os(buf.data(), (unsigned)cap);
        UINT64 size = in_len;
        if (cc.Code(&is, &os, &size) != S_OK || os.overflow_get()) return FZ_ELIB;
        size_t n = (size_t)os.size_get();
        unsigned char *r = (unsigned char *)malloc(n ? n : 1);
        if (!r) return FZ_ENOMEM;
        memcpy(r, buf.data(), n);
        *out = r;
        *out_len = n;
        return FZ_OK;
    } catch (const std::bad_alloc &) {
        return FZ_ENOMEM;
    } catch (...) {
        return FZ_ELIB;
    }
}
