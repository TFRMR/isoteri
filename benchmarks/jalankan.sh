#!/usr/bin/env bash
# Jalankan semua benchmark & catat waktu wall-clock kasar.
# Bukan pengukuran ilmiah presisi (belum warm-up run, belum banyak sampel) --
# tujuannya cuma supaya SETIAP perubahan compiler bisa langsung ketahuan
# bikin Isoteri lebih cepat atau lebih lambat, sesuai permintaan di docs/IR.md.
#
# Pakai: bash benchmarks/jalankan.sh [path/ke/binary/isoteri]
set -euo pipefail
BIN="${1:-./target/release/isoteri}"
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ ! -x "$BIN" ]; then
  echo "Binary tidak ditemukan: $BIN (jalankan 'cargo build --release' dulu, atau kasih path lewat argumen)"
  exit 1
fi

for f in "$DIR"/*.iso; do
  nama=$(basename "$f")
  mulai=$(date +%s.%N)
  hasil=$("$BIN" "$f")
  selesai=$(date +%s.%N)
  waktu=$(echo "$selesai - $mulai" | bc)
  printf "%-24s %8.3fs   hasil: %s\n" "$nama" "$waktu" "$hasil"
done
