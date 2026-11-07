#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
mkdir -p .build/abi
cargo build -p buntrail-ffi --offline --message-format=json > .build/abi/build.json
out=$(python3 -c 'import json,sys; rows=[json.loads(s) for s in open(sys.argv[1])]; print(next(r["out_dir"] for r in rows if r.get("reason")=="build-script-executed" and "buntrail-ffi" in r["package_id"]))' .build/abi/build.json)
cmp include/buntrail.h "$out/buntrail.h"
target=$(cargo metadata --no-deps --format-version 1 --offline | python3 -c 'import json,sys;print(json.load(sys.stdin)["target_directory"])')
case "$(uname -s)" in
  Darwin) libs='-framework Security -framework CoreServices -framework CoreFoundation -framework SystemConfiguration' ;;
  Linux) libs='-lpthread -ldl -lm' ;;
  *) echo 'Native Windows C/C++ ABI checks remain unverified.' >&2; exit 2 ;;
esac
# Fixed platform linker flags intentionally undergo word splitting.
cc -I include tests/abi/smoke.c "$target/debug/libbuntrail_ffi.a" $libs -o .build/abi/c-smoke
c++ -std=c++17 -pthread -I include tests/abi/smoke.cpp "$target/debug/libbuntrail_ffi.a" $libs -o .build/abi/cpp-smoke
.build/abi/c-smoke
.build/abi/cpp-smoke
