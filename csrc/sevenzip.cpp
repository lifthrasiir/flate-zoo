/* params: level, passes, fb, mc, algo
 * level < 0 (or absent) means 7-Zip's default; passes/fb/mc/algo of 0 (algo: -1) mean "derived from level"
 * exactly as in `7z a -tzip -mx=<level> [-mpass=.. -mfb=.. -mmc=.. -ma=..]` (CEncProps::Normalize). */
#include "StdAfx.h"
#include "../../Common/MyInitGuid.h"
#include "../../../C/Alloc.h"
#include "../../Common/MyCom.h"
#include "../ICoder.h"
#include "DeflateEncoder.h"
#include "fz.h"
#include <vector>
#include <new>

namespace {

class CMemIn Z7_final : public ISequentialInStream, public CMyUnknownImp {
  Z7_COM_UNKNOWN_IMP_1(ISequentialInStream)
  Z7_IFACE_COM7_IMP(ISequentialInStream)
  const Byte *_p; size_t _rem;
public:
  CMemIn(const Byte *p, size_t n) : _p(p), _rem(n) {}
};

Z7_COM7F_IMF(CMemIn::Read(void *data, UInt32 size, UInt32 *processed)) {
  size_t n = _rem < size ? _rem : size;
  if (n) memcpy(data, _p, n);
  _p += n; _rem -= n;
  if (processed) *processed = (UInt32)n;
  return S_OK;
}

class CMemOut Z7_final : public ISequentialOutStream, public CMyUnknownImp {
  Z7_COM_UNKNOWN_IMP_1(ISequentialOutStream)
  Z7_IFACE_COM7_IMP(ISequentialOutStream)
public:
  std::vector<Byte> buf;
};

Z7_COM7F_IMF(CMemOut::Write(const void *data, UInt32 size, UInt32 *processed)) {
  const Byte *p = (const Byte *)data;
  buf.insert(buf.end(), p, p + size);
  if (processed) *processed = size;
  return S_OK;
}

}

extern "C" int FZ_ENTRY(run)(FZ_ARGS) {
  if (nparams != 5) return FZ_EPARAM;
  CMyComPtr<ICompressCoder> coder = new NCompress::NDeflate::NEncoder::CCOMCoder;
  CMyComPtr<ICompressSetCoderProperties> setter;
  coder.QueryInterface(IID_ICompressSetCoderProperties, &setter);
  if (!setter) return FZ_ELIB;
  PROPID ids[5]; PROPVARIANT vals[5]; UInt32 n = 0;
  auto add = [&](PROPID id, long long v) {
    ids[n] = id; vals[n].vt = VT_UI4; vals[n].ulVal = (UInt32)v; n++;
  };
  if (params[0] >= 0) add(NCoderPropID::kLevel, params[0]);
  if (params[1] > 0) add(NCoderPropID::kNumPasses, params[1]);
  if (params[2] > 0) add(NCoderPropID::kNumFastBytes, params[2]);
  if (params[3] > 0) add(NCoderPropID::kMatchFinderCycles, params[3]);
  if (params[4] >= 0) add(NCoderPropID::kAlgorithm, params[4]);
  try {
    if (setter->SetCoderProperties(ids, vals, n) != S_OK) return FZ_EPARAM;
    CMemIn *src = new CMemIn(in_len ? in : (const unsigned char *)"", in_len);
    CMemOut *outs = new CMemOut;
    CMyComPtr<ISequentialInStream> inRef = src;
    CMyComPtr<ISequentialOutStream> outRef = outs;
    UInt64 size = in_len;
    HRESULT hr = coder->Code(src, outs, &size, NULL, NULL);
    if (hr == E_OUTOFMEMORY) return FZ_ENOMEM;
    if (hr != S_OK) return FZ_ELIB;
    unsigned char *r = (unsigned char *)malloc(outs->buf.size() ? outs->buf.size() : 1);
    if (!r) return FZ_ENOMEM;
    memcpy(r, outs->buf.data(), outs->buf.size());
    *out = r;
    *out_len = outs->buf.size();
    return FZ_OK;
  } catch (const std::bad_alloc &) {
    return FZ_ENOMEM;
  } catch (...) {
    return FZ_ELIB;
  }
}
