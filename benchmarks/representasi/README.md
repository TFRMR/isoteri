# Eksperimen "Type Information -> Representation" (item #6 ROADMAP)

Bagian dari rencana besar item #6 ROADMAP.md: perluas kelayakan JIT
Cranelift, dimulai dari langkah paling gampang (struct/bentuk numerik).
Prinsip arsitektur yang dipakai (disepakati bareng): **jangan bikin V8**
-- pakai Cranelift/VM/JIT/AOT yang sudah ada, optimalkan bagian yang
memang jadi keunggulan Isoteri: alur `type information -> representation
-> optimization -> runtime semantics`. Target milestone bertahap:
**30x -> <=5x -> <=2x -> ~1x** lebih lambat dari Node.js (bukan klaim
"secepat" langsung, realistis bertahap).

## Yang Sudah Ditemukan & Diperbaiki di Putaran Ini

### Temuan: infrastruktur "flatten struct numerik" SUDAH SEBAGIAN ADA

`bentuk` (struct) di Isoteri sudah bisa punya anotasi tipe field sejak
lama (`bentuk Titik { x: Angka, y: Angka }`), dan resolver sudah
mendeteksi kalau SEMUA field sebuah struct numerik (`hitung_param_flat`,
`param_flat_info` di `src/lib.rs`) -- struct seperti itu, kalau dipakai
sebagai PARAMETER fungsi, akses field-nya (`p.x`) langsung diterjemahkan
ke slot lokal biasa saat kompilasi (`CExpr::Local`), BUKAN lewat
`CExpr::Field` dinamis. Infrastruktur ini sudah ada, cuma belum lengkap.

### Perbaikan: fast-path buat literal struct langsung di titik panggil

**Sebelum**: `f(Titik{x:3,y:4})` tetap membangun `Instans` dinamis dulu
(`Instr::BuatInstans`, alokasi heap) lalu LANGSUNG dibongkar lagi lewat
mekanisme `SimpanLaluField` -- kerja dua kali buat objek yang cuma hidup
sepersekian detik.

**Sesudah**: kalau argumen adalah literal `Bentuk` langsung (bukan lewat
variabel), compiler skip konstruksi `Instans` sepenuhnya -- field-nya
(sudah diurutkan sesuai skema lewat `urutkan_field_bentuk()`) langsung
jadi argumen. Diimplementasikan di KEDUA resolver (global & lokal, lihat
`src/lib.rs`, dekat komentar "Fast-path: argumen literal Bentuk").

**Hasil terverifikasi** (isolated micro-test, `jarak_kuadrat(Titik{x:3,y:4})`
dipanggil 2 juta kali dalam loop):

| | Sebelum fix | Sesudah fix |
|---|---:|---:|
| Fungsi dgn parameter struct | 1.08s | **0.52s** (~2x lebih cepat) |
| Fungsi dgn 2 parameter angka biasa (baseline) | 0.82s | 0.82s (tidak berubah) |

Setelah fix, versi struct malah SEDIKIT LEBIH CEPAT dari baseline
2-parameter biasa -- konsisten di 5x pengulangan. 14/14 test regresi
tetap lulus (`scripts/regresi.sh`).

**PENTING -- luruskan skop perbaikan ini**: ini optimasi level
INTERPRETER/bytecode (mengurangi kerja yang terbuang), BUKAN "sekarang
fungsi ber-parameter struct dikompilasi native lewat Cranelift JIT".
Field `tipe_jit: Option<TipeJit>` di `CFungsi` (yang menentukan
kelayakan Cranelift JIT sungguhan) **masih secara eksplisit menolak**
`Bentuk` (lihat komentar di kode: "tanpa Teks/Bool/Daftar/Peta/Bentuk").
Perbaikan di putaran ini TIDAK mengubah itu.

## Kenapa `validasi_petani_struct` (Benchmark Representatif) Masih Lambat

Ditulis ulang `validasi_petani` (lihat `benchmarks/head_to_head/`) pakai
`bentuk DataPetani` (bukan `Peta`) buat ukur seberapa dekat ke target
milestone. Hasil jujur:

| | Isoteri AOT | Node.js |
|---|---:|---:|
| `validasi_petani_struct` (500rb validasi) | ~840ms | ~42ms |

**Masih ~20x lebih lambat** -- BELUM sampai target `<=5x`. Analisis:

1. Fungsi `validasi_petani_struct()` sendiri masih TIDAK ELIGIBLE buat
   `tipe_jit` (dikompilasi Cranelift native) karena eksklusi eksplisit
   `Bentuk` di atas -- fast-path yang kita tambahkan cuma bikin
   INTERPRETER-nya lebih hemat, bukan bikin fungsinya jadi kode native.
2. Fungsi pembungkus (`validasi_satu`) dan loop top-level pakai variabel
   GLOBAL (`ingat n`, `ingat i` di level atas program) DAN operator `%`
   (modulo) -- keduanya JUGA di luar syarat `cek_jit_murni_stmt`/
   `cek_jit_murni_nilai` (cuma `Tambah`/`Kurang`/`Kali` yang diizinkan
   di ekspresi "murni", modulo tidak termasuk; akses variabel global
   langsung didiskualifikasi).

## Langkah Selanjutnya (Belum Dikerjakan di Putaran Ini)

Supaya benar-benar sampai ke kode native (bukan cuma interpreter lebih
hemat), perlu 2 pekerjaan LANJUTAN yang lebih besar:

1. **Perluas `tipe_jit` inference supaya menerima `Bentuk` numerik** --
   bukan cuma `param_flat` (mekanisme terpisah yang sudah ada), tapi
   benar-benar diwariskan ke Cranelift codegen: tiap field jadi satu
   nilai native (I64/F64) di dalam fungsi yang dikompilasi, sama seperti
   parameter skalar biasa sekarang.
2. **Perluas operator & konteks yang diizinkan** di `cek_jit_murni_*` --
   minimal modulo (`%`) dan pembagian dengan pengecekan div-by-zero, dan
   PALING PENTING: cara supaya fungsi yang MEMANGGIL fungsi ber-tipe-JIT
   dari dalam konteks yang pakai variabel global tetap bisa manfaat
   (skema hybrid: bagian yang JIT-elig dikompilasi native, bagian lain
   tetap interpreter, TANPA harus SELURUH fungsi pembungkusnya jadi
   elig).

Ini scope pekerjaan MINGGUAN (compiler engineering signifikan), sesuai
catatan di ROADMAP.md item #6 -- perbaikan di putaran ini adalah
langkah AWAL yang nyata & terverifikasi (2x di level interpreter),
bukan solusi penuh.

## Putaran Lanjutan: `TipeJit::Campur` -- Native JIT Sungguhan buat Struct Campuran

Item #1 di "Langkah Selanjutnya" di atas SEKARANG SELESAI: `bentuk`
dengan field campuran Angka+Desimal (bukan cuma Bentuk-seragam-tipe)
sekarang BISA benar-benar dikompilasi native Cranelift, bukan cuma
optimasi interpreter.

### Pendekatan: aman lewat verifikasi per-operasi, bukan promosi tipe implisit

Ditambah varian baru `TipeJit::Campur` (lihat catatan panjang di
`src/lib.rs`) -- fungsi dengan slot BERBEDA tipe per field, TAPI setiap
OPERASI individual (perbandingan `==`, `<=`, dst) diverifikasi
same-type di kedua operand-nya (`tipe_cexpr()`) SEBELUM diizinkan JIT.
Kalau ada SATU operasi saja yang benar-benar mencampur Angka+Desimal
(mis. `d.a == d.b` langsung), seluruh fungsi GAGAL syarat murni --
fallback ke interpreter (aman, hasil tetap benar), BUKAN nyoba promosi
tipe implisit (int->float) yang lebih riskan salah kalau meleset.

**Skop SENGAJA dipersempit buat keamanan**: mode Campur cuma didukung
buat PERBANDINGAN, TIDAK BOLEH aritmatika (`+`/`-`/`*`) sama sekali --
menghindari kebutuhan mekanisme overflow-flag/promosi-tipe yang jauh
lebih riskan. Nilai kembalian (`kembalikan`) WAJIB Angka (signature
Cranelift butuh SATU tipe kembalian pasti). Juga cuma didukung di jalur
`kompilasi_dari_ir` (via-ir/AOT) -- jalur JIT legacy (`isoteri
prog.iso` default) SENGAJA menolak Campur di awal (fallback interpreter
otomatis, tetap benar, cuma tidak dapat manfaat native compile di jalur
itu).

### Verifikasi correctness

15/15 test regresi lulus (`scripts/regresi.sh`), termasuk test case
baru `tes_regresi/tipe_campur_jit.iso` yang mengunci:
- Struct dengan field Angka DAN Desimal bekerja benar sekaligus (bukan
  cuma salah satu tipe)
- **Kasus paling penting**: operasi yang BENAR-BENAR mencampur tipe
  (`d.a == d.b`, Angka dibanding LANGSUNG dengan Desimal) tetap dapat
  hasil BENAR lewat fallback interpreter otomatis -- dibuktikan
  `tipe_jit_final=None` buat fungsi itu spesifik (bukan `Some(Campur)`
  seperti fungsi yang aman), verified via `ISOTERI_DEBUG_JIT=1`.
- Konsisten di 3 jalur eksekusi (JIT default, bytecode murni, via-ir),
  dengan 1 divergensi stderr yang diizinkan & didokumentasikan
  (`tes_regresi/divergensi_diketahui.txt`) -- warning informational
  "Campur belum didukung di jalur legacy" yang cuma tercetak di mode
  JIT default (karena sempat coba dulu sebelum fallback), TIDAK ada di
  bytecode murni (skip percobaan sama sekali) maupun via-ir (berhasil
  native, tidak perlu fallback).

### Hasil benchmark (angka jujur, bukan cherry-pick)

`validasi_petani_struct` sekarang MENANG di level fungsi individual
(`validasi_petani_struct()` sendiri dapat `tipe_jit_final=Some(Campur)`
-- native compile berhasil), tapi fungsi PEMBUNGKUS (`validasi_satu`,
dipanggil 500rb kali di top-level loop) MASIH interpreter penuh --
parameternya (`i`) tidak dianotasi tipe DAN pakai operator `%` (modulo)
yang belum didukung sama sekali di JIT manapun (bukan spesifik Campur --
`Bagi`/`Modulo` keduanya masih `unreachable!()` di semua mode, perlu
mekanisme pengecekan pembagi-nol di codegen yang belum dibangun).

| | Sebelum (fast-path literal saja) | Sesudah (+ mode Campur) | Node.js |
|---|---:|---:|---:|
| `validasi_petani_struct` AOT | ~840ms | **~500ms** (~40% lebih cepat) | ~42ms |

**Masih ~12x lebih lambat dari Node.js -- BELUM sampai target `<=5x`**,
tapi progress nyata & terukur (840ms->500ms). Sisa jaraknya SEKARANG
punya penyebab yang jelas & sempit: `validasi_satu` (fungsi
pembungkus) perlu (a) anotasi tipe pada parameternya, DAN (b) dukungan
`%` (modulo) di JIT -- keduanya independen dari pekerjaan mode Campur
di atas, item terpisah buat putaran berikutnya.

