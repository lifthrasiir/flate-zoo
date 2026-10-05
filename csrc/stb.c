/* params: quality (hash chain length, >= 5) */
#include "fz.h"
#include "stb_image_write.h"

/* only declared in the implementation section */
unsigned char *stbi_zlib_compress(unsigned char *data, int data_len, int *out_len, int quality);

int FZ_ENTRY(run)(FZ_ARGS) {
    if (nparams != 1 || in_len > 0x7fffffff) return FZ_EPARAM;
    int n = 0;
    unsigned char *z = stbi_zlib_compress((unsigned char *)in, (int)in_len, &n, (int)params[0]);
    return fz_from_zlib(z, (size_t)n, out, out_len);
}
