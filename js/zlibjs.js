// params: type (0 stored, 1 fixed, 2 dynamic), lazy
function __run(d, type, lazy) {
  return new Zlib.RawDeflate(d, { compressionType: type, lazy: lazy }).compress();
}
