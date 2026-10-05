/* params: level, wbits, detect -- Crypto++ Deflator(attachment, level, log2WindowSize, detect) */
#include "fz.h"
#include "zdeflate.h"
#include "filters.h"
#include <string>

/* The library normally gets these from dll.cpp/integer.cpp, which drag in most of Crypto++
 * (big integers, ASN.1, hashes). The Deflator never needs the Integer conversion, so provide
 * the explicit template instantiations and trivial stand-ins here instead. */
namespace CryptoPP {
template class AlgorithmParametersTemplate<bool>;
template class AlgorithmParametersTemplate<int>;
template class AlgorithmParametersTemplate<ConstByteArrayParameter>;
template class StringSinkTemplate<std::string>;

bool AssignIntToInteger(const std::type_info &, void *, const void *) {
    return false; /* the target is never an Integer here */
}
template <> std::string IntToString<unsigned long>(unsigned long value, unsigned int base) {
    std::string s;
    do { s.insert(s.begin(), "0123456789abcdefghijklmnopqrstuvwxyz"[value % base]); value /= base; } while (value);
    return s;
}
}

extern "C" int FZ_ENTRY(run)(FZ_ARGS) {
    if (nparams != 3) return FZ_EPARAM;
    try {
        std::string res;
        {
            CryptoPP::Deflator d(new CryptoPP::StringSink(res), (int)params[0], (int)params[1],
                                 params[2] != 0);
            d.Put(in, in_len);
            d.MessageEnd();
        }
        unsigned char *r = (unsigned char *)malloc(res.size() ? res.size() : 1);
        if (!r) return FZ_ENOMEM;
        memcpy(r, res.data(), res.size());
        *out = r;
        *out_len = res.size();
        return FZ_OK;
    } catch (const CryptoPP::InvalidArgument &) {
        return FZ_EPARAM;
    } catch (const std::bad_alloc &) {
        return FZ_ENOMEM;
    } catch (...) {
        return FZ_ELIB;
    }
}
