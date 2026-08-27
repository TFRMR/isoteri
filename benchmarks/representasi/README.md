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
