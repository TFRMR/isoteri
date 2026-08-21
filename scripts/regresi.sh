#!/usr/bin/env bash
# Regression suite otomatis buat compiler Isoteri sendiri (BUKAN `isoteri uji`, yang itu buat
# nguji program Isoteri milik pengguna lewat gagal_uji() -- lihat main.rs mode_uji). Tujuan
# script ini: gantikan kebiasaan manual "diuji terhadap N program contoh" tiap sesi dengan satu
# perintah yang otomatis nangkep DUA jenis regresi sekaligus:
#
#   1. Regresi terhadap hasil yang sudah diketahui benar (golden file .out) -- kalau ada
#      perubahan compiler yang bikin hasil program berubah diam-diam, ketahuan langsung.
#   2. Divergensi ANTAR JALUR EKSEKUSI untuk program yang SAMA -- bytecode murni
#      (ISOTERI_NO_JIT=1), JIT produksi (`isoteri prog.iso`), dan via-ir (`isoteri via-ir
#      prog.iso`, jalur yang sama dipakai `isoteri bangun`/AOT). Ketiganya SEHARUSNYA selalu
#      kasih hasil identik untuk program yang sama -- kalau beda, itu bug, walau golden file-nya
#      belum ada / programnya baru. Ini persis metodologi manual yang nemuin bug wrap-around
#      overflow JIT sesi lalu (lihat KETERBATASAN.md), sekarang jalan otomatis tiap kali dipanggil.
#
# Pakai:
#   bash scripts/regresi.sh                 jalankan semua kasus di tes_regresi/, exit 1 kalau ada yang gagal
#   bash scripts/regresi.sh --perbarui       TULIS ULANG semua golden file .out dari hasil SEKARANG
#                                            (pakai HATI-HATI -- cuma kalau kamu SUDAH verifikasi manual
#                                            hasil barunya benar, mis. abis nambah fitur baru yang
#                                            sengaja ubah perilaku/nambah kasus uji baru)
#   bash scripts/regresi.sh --perbarui nama_kasus   perbarui golden file SATU kasus uji saja
#
# Setiap kasus uji = satu file tes_regresi/<nama>.iso, golden file-nya tes_regresi/<nama>.out
# (dibuat otomatis kalau belum ada). Konten .out = stdout gabung stderr + baris terakhir
# "EXIT:<kode>" -- exit code ikut dibandingkan supaya error yang HARUSNYA muncul (mis. kasus
# overflow_*.iso, exit 1) gak kebablasan dianggap "lulus" gara-gara cuma banding teksnya doang.

set -uo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$DIR/target/release/isoteri"
TES_DIR="$DIR/tes_regresi"
ALLOWLIST="$TES_DIR/divergensi_diketahui.txt"

# Format tiap baris: nama_kasus|jalur|alasan (jalur: bytecode atau via-ir -- selalu dibandingkan
# TERHADAP jit produksi, jadi cuma perlu 1 sisi). Baris kosong / diawali # diabaikan. Cuma buat
# divergensi yang SUDAH diverifikasi manual sebagai "beda tapi sama-sama benar" (mis. wording
# pesan error beda tapi dua-duanya tetap error jelas & catchable) -- BUKAN tempat nyembunyiin
# silent-wrong-value semacam bug wrap-around overflow JIT dulu. Kalau ragu, JANGAN ditambah ke
# sini -- biarin GAGAL supaya ketauan & diinvestigasi manual dulu.
diizinkan() {
  local nama="$1" jalur="$2"
  [ -f "$ALLOWLIST" ] || return 1
  while IFS='|' read -r n j _; do
    [[ "$n" =~ ^#.*$ || -z "$n" ]] && continue
    if [ "$n" = "$nama" ] && [ "$j" = "$jalur" ]; then return 0; fi
  done < "$ALLOWLIST"
  return 1
}

if [ ! -x "$BIN" ]; then
  echo "Binary tidak ditemukan: $BIN -- jalankan 'cargo build --release' dulu."
  exit 1
fi
if [ ! -d "$TES_DIR" ]; then
  echo "Direktori $TES_DIR tidak ditemukan."
  exit 1
fi

PERBARUI=0
FILTER=""
if [ "${1:-}" = "--perbarui" ]; then
  PERBARUI=1
  FILTER="${2:-}"
fi

# Jalankan satu program lewat satu jalur eksekusi, balikin "<stdout+stderr>\nEXIT:<kode>".
jalankan_satu() {
  local mode="$1" file="$2" keluaran kode
  case "$mode" in
    jit)      keluaran=$("$BIN" "$file" 2>&1); kode=$? ;;
    bytecode) keluaran=$(ISOTERI_NO_JIT=1 "$BIN" "$file" 2>&1); kode=$? ;;
    via-ir)   keluaran=$("$BIN" via-ir "$file" 2>&1); kode=$? ;;
  esac
  printf '%s\nEXIT:%d' "$keluaran" "$kode"
}

total=0
lulus=0
gagal=0
gagal_nama=()

for f in "$TES_DIR"/*.iso; do
  [ -e "$f" ] || continue
  nama=$(basename "$f" .iso)
  if [ -n "$FILTER" ] && [ "$nama" != "$FILTER" ]; then continue; fi
  total=$((total + 1))
  out_file="$TES_DIR/$nama.out"

  hasil_jit=$(jalankan_satu jit "$f")
  hasil_bc=$(jalankan_satu bytecode "$f")
  hasil_ir=$(jalankan_satu via-ir "$f")

  masalah=""
  catatan=""

  # 1. Cek tiga jalur eksekusi saling konsisten (bug class: silent JIT/bytecode/via-ir divergence).
  if [ "$hasil_jit" != "$hasil_bc" ]; then
    if diizinkan "$nama" "bytecode"; then
      catatan="${catatan:-}  (divergensi bytecode DIIZINKAN -- lihat divergensi_diketahui.txt)\n"
    else
      masalah="${masalah}  - JIT produksi vs bytecode murni (ISOTERI_NO_JIT=1) BEDA HASIL:\n$(diff <(echo "$hasil_bc") <(echo "$hasil_jit") | sed 's/^/      /')\n"
    fi
  fi
  if [ "$hasil_jit" != "$hasil_ir" ]; then
    if diizinkan "$nama" "via-ir"; then
      catatan="${catatan:-}  (divergensi via-ir DIIZINKAN -- lihat divergensi_diketahui.txt)\n"
    else
      masalah="${masalah}  - JIT produksi vs via-ir BEDA HASIL:\n$(diff <(echo "$hasil_ir") <(echo "$hasil_jit") | sed 's/^/      /')\n"
    fi
  fi

  if [ "$PERBARUI" = "1" ]; then
    if [ -n "$masalah" ]; then
      echo "PERINGATAN $nama: jalur eksekusi saling beda hasil, TETAP ditulis golden dari jalur JIT produksi -- cek manual dulu:"
      echo -e "$masalah"
    fi
    echo "$hasil_jit" > "$out_file"
    echo "diperbarui: $nama.out"
    lulus=$((lulus + 1))
    continue
  fi

  # 2. Cek terhadap golden file (bug class: regresi perilaku vs baseline yang sudah diverifikasi benar).
  if [ ! -f "$out_file" ]; then
    masalah="${masalah}  - Belum ada golden file ($nama.out). Jalankan dengan --perbarui SETELAH verifikasi manual hasilnya benar.\n"
  else
    golden=$(cat "$out_file")
    if [ "$hasil_jit" != "$golden" ]; then
      masalah="${masalah}  - Hasil (jalur JIT produksi) BEDA dari golden file:\n$(diff <(echo "$golden") <(echo "$hasil_jit") | sed 's/^/      /')\n"
    fi
  fi

  if [ -z "$masalah" ]; then
    echo "LULUS  $nama"
    [ -n "$catatan" ] && echo -e "$catatan"
    lulus=$((lulus + 1))
  else
    echo "GAGAL  $nama"
    echo -e "$masalah"
    gagal=$((gagal + 1))
    gagal_nama+=("$nama")
  fi
done

echo "---"
if [ "$PERBARUI" = "1" ]; then
  echo "$lulus golden file diperbarui dari $total kasus."
  exit 0
fi
echo "$lulus lulus, $gagal gagal, dari $total kasus regresi."
if [ "$gagal" -gt 0 ]; then
  echo "Kasus gagal: ${gagal_nama[*]}"
  exit 1
fi
exit 0
