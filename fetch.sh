#!/bin/sh
# Fetches pinned third-party DEFLATE implementations into vendor/.
# Re-running is idempotent: existing directories are left alone.
set -eu
cd "$(dirname "$0")"
mkdir -p vendor
V=vendor

git_at() { # name url ref
  [ -d "$V/$1" ] && return 0
  echo "fetching $1 @ $3" >&2
  tmp="$V/.$1.tmp"; rm -rf "$tmp"
  git init -q "$tmp"
  git -C "$tmp" fetch -q --depth 1 "$2" "$3"
  git -C "$tmp" checkout -q FETCH_HEAD
  git -C "$tmp" rev-parse HEAD > "$tmp/.fetched-rev"
  rm -rf "$tmp/.git"; mv "$tmp" "$V/$1"
}
tar_at() { # name url [strip]
  [ -d "$V/$1" ] && return 0
  echo "fetching $1 from $2" >&2
  tmp="$V/.$1.tmp"; rm -rf "$tmp"; mkdir -p "$tmp"
  curl -fsSL "$2" | tar -xf - -C "$tmp" --strip-components "${3:-1}"
  echo "$2" > "$tmp/.fetched-rev"
  mv "$tmp" "$V/$1"
}
npm_at() { # name package@version
  [ -d "$V/$1" ] && return 0
  echo "fetching $1 ($2) from npm" >&2
  tmp="$V/.$1.tmp"; rm -rf "$tmp"; mkdir -p "$tmp"
  curl -fsSL "$(npm view "$2" dist.tarball)" | tar -xzf - -C "$tmp" --strip-components 1
  echo "$2" > "$tmp/.fetched-rev"
  mv "$tmp" "$V/$1"
}

git_at zlib          https://github.com/madler/zlib                v1.3.1
git_at zlib-ng       https://github.com/zlib-ng/zlib-ng            2.3.3
git_at zlib-chromium https://chromium.googlesource.com/chromium/src/third_party/zlib 456ae730017b9b4bd3371927abc82627fd51feef
git_at zlib-cloudflare https://github.com/cloudflare/zlib          3944b7d59cf50aa144d8bba69809edea072161d4
git_at libdeflate    https://github.com/ebiggers/libdeflate        v1.26
git_at zopfli        https://github.com/google/zopfli              df1517dd8518113a245e4d203a04b8a0e02ab493
git_at miniz         https://github.com/richgel999/miniz           3.1.2
git_at isa-l         https://github.com/intel/isa-l                v2.32.1
git_at advancecomp   https://github.com/amadvance/advancecomp      v2.6
git_at cryptopp      https://github.com/weidai11/cryptopp          CRYPTOPP_8_9_0
git_at 7zip          https://github.com/ip7z/7zip                  26.03
git_at stb           https://github.com/nothings/stb               2c980bb59875b0d32144a71867fbdebb2f77cd20
git_at lodepng       https://github.com/lvandeve/lodepng           ff206aa97444bd39fad60ebaae7a959c2b88ba83
git_at vurtun-lib    https://github.com/vurtun/lib                 5a3f3aba052e63ffae8eb0214c6bb8ffffedea3c
git_at uzlib         https://github.com/pfalcon/uzlib              6d60d651a4499a64f2e5b21b4cc08d98cb84b5c1
git_at libslz        https://github.com/wtarreau/libslz            daf74c4b26ab77a0306b81d5ed5f1e7a0c3aa81a
git_at ect           https://github.com/fhanau/Efficient-Compression-Tool e711c5ea9d725d02db546ce926a66b91b68ecb3a
tar_at gzip          https://ftp.gnu.org/gnu/gzip/gzip-1.14.tar.xz
tar_at infozip       http://deb.debian.org/debian/pool/main/z/zip/zip_3.0.orig.tar.gz
npm_at js-fflate     fflate@0.8.2
npm_at js-pako       pako@2.1.0
npm_at js-uzip       uzip@0.20201231.0
npm_at js-zlibjs     zlibjs@0.3.1

(cd go && go mod vendor)
