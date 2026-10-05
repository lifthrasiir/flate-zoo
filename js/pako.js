// params: level, wbits, mem, strategy (as zlib's deflateInit2)
function __run(d, level, wbits, mem, strategy) {
  return pako.deflateRaw(d, { level: level, windowBits: wbits, memLevel: mem, strategy: strategy });
}
