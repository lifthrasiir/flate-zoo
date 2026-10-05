// params: level, mem (-1 = fflate's size-dependent default)
function __run(d, level, mem) {
  var o = { level: level };
  if (mem >= 0) o.mem = mem;
  return fflate.deflateSync(d, o);
}
