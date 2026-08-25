# Benchmark Head-to-Head: Isoteri (AOT) vs Node.js vs Python

Ini adalah implementasi item **"Prasyarat #4"** di `ROADMAP.md` -- benchmark
backend Isoteri (AOT) vs Node.js/Python untuk beban kerja representatif,
supaya klaim "lebih cepat di backend" (lihat bagian "Arah strategis" di
ROADMAP.md) punya angka publik, bukan janji.

## Cara menjalankan ulang

```bash
# 1. Build binary AOT (sekali saja, ~1 detik kalau cache cargo sudah ada,
#    ~4 menit kalau dari nol)
cd isoteri-repo
./target/release/isoteri bangun benchmarks/head_to_head/isoteri/fib_rekursif.iso \
    -o benchmarks/head_to_head/isoteri/fib_rekursif_aot
./target/release/isoteri bangun benchmarks/head_to_head/isoteri/daftar_operasi.iso \
    -o benchmarks/head_to_head/isoteri/daftar_operasi_aot
./target/release/isoteri bangun benchmarks/head_to_head/isoteri/validasi_petani.iso \
    -o benchmarks/head_to_head/isoteri/validasi_petani_aot

# 2. Jalankan harness (butuh python3, node di PATH)
cd benchmarks/head_to_head
python3 jalankan_benchmark.py --sampel 10
```

Hasil tersimpan di `hasil/HASIL.md` (ringkasan) dan `hasil/hasil_mentah.json`
(semua sampel mentah, buat analisis lebih lanjut).

## Metodologi

- **Tiga workload**, tiap satu diimplementasikan ULANG (bukan auto-generate)
  di tiga bahasa dengan logika identik: `isoteri/*.iso`, `node/*.js`,
  `python/*.py`.
- **Output diverifikasi identik** di ketiga bahasa sebelum sampling waktu
  dimulai -- kalau outputnya beda, benchmark itu digagalkan otomatis oleh
  harness (`jalankan_benchmark.py`), bukan diam-diam dipakai.
- **Wall-clock end-to-end per proses** (subprocess baru tiap sampel,
  termasuk startup interpreter/runtime) -- ini SENGAJA, bukan loop panas di
  dalam satu proses yang sama. Alasan: skenario nyata yang relevan buat
  Isoteri (lihat "Arah strategis" ROADMAP.md) adalah request singkat di
  backend (CLI/handler pendek), bukan proses long-running yang jalan
  berhari-hari. Startup cost itu sendiri adalah bagian dari perbandingan
  yang jujur.
- **1 run pemanasan dibuang**, lalu **10 sampel** diambil (median, min, max,
  stdev dilaporkan). Median dipakai buat perbandingan utama karena tahan
  outlier (mis. GC pause kebetulan, noise OS scheduler).
- **Isoteri diuji dalam mode AOT** (`isoteri bangun`, binary native mandiri)
  -- BUKAN mode JIT/bytecode interaktif (`isoteri program.iso`) -- karena
  ini yang diklaim di ROADMAP.md sebagai jalur "beneran lebih cepat" untuk
  backend.
- Semua dijalankan di mesin & sesi yang sama secara berurutan (bukan
  paralel) untuk menghindari kontensi CPU antar-proses yang mengganggu
  pengukuran.

## Tiga workload, dan kenapa dipilih

1. **`validasi_petani`** -- logika bisnis NYATA (bukan sintetis), diambil
   langsung dari `contoh_satu_skema/skema_petani.iso` yang sudah
   tervalidasi jadi jembatan "satu skema, dua sisi". Dijalankan 500.000
   kali atas dataset sintetis deterministik (pola valid/invalid berulang
   tiap 5 baris, supaya hasil akhirnya bisa dicek exact match). Ini
   simulasi paling representatif dari kasus pakai utama Isoteri:
   validasi request API/form dalam jumlah besar.
2. **`fib_rekursif`** -- `fib(32)` rekursif naif. CPU-bound murni, tanpa
   alokasi objek/list sama sekali -- mengukur overhead pemanggilan fungsi &
   aritmatika dasar seteliti mungkin, terlepas dari struktur data.
3. **`daftar_operasi`** -- bangun daftar 20.000 elemen, `petakan` (map),
   `saring` (filter), lalu jumlahkan. Simulasi pemrosesan data hasil panen
   dalam skala menengah.

## Hasil (ringkasan -- lihat `hasil/HASIL.md` untuk angka lengkap)

| Workload | Pemenang | Margin | Status |
|---|---|---|---|
| `fib_rekursif` | **Isoteri (AOT)** | 2.4x lebih cepat dari Node.js, 12x dari Python | Sudah cepat sejak awal |
| `validasi_petani` | Node.js | Isoteri 31x LEBIH LAMBAT dari Node.js, 6x lebih lambat dari Python | **MASIH BOTTLENECK** -- lihat bagian analisis |
| `daftar_operasi` | **Isoteri (AOT)** vs Node.js | Isoteri 1.6x lebih cepat dari Node, Python 1.2x lebih cepat dari Isoteri | **DIPERBAIKI** -- dulu kalah 130-260x, lihat catatan di bawah |

**Ini bukan salah ketik.** Isoteri AOT menang telak di komputasi murni
(`fib_rekursif`), tapi kalah jauh di dua workload lain. Ini temuan yang
dilaporkan APA ADANYA sesuai prinsip "klaim lebih cepat yang jujur" di
ROADMAP.md -- bukan cherry-picking hasil yang bagus saja.

## Analisis: kenapa hasilnya timpang begini?

Investigasi cepat (lihat riwayat kerja) mengisolasi penyebabnya jadi dua
karakteristik nyata Isoteri saat ini, BUKAN bug di benchmark. Yang pertama
(`gabung()`) SUDAH DIPERBAIKI setelah benchmark ini pertama kali
dijalankan -- lihat status di tiap bagian di bawah.

### 1. `gabung()` (list append) bersifat immutable -- SUDAH DIPERBAIKI ✅

**Temuan awal**: Didokumentasikan di `docs/REFERENSI.md`, `gabung(daftar,
item)` "kembalikan Daftar BARU dengan item ditambahkan di akhir" -- artinya
tiap panggilan meng-copy seluruh isi list sejauh ini. Build list N elemen
lewat `gabung()` di dalam loop itu O(n) per panggilan / **O(n^2) total**.
Ini penyebab utama `daftar_operasi` awalnya kalah 130-260x dari Node.js/
Python.

**Perbaikan**: Ditambahkan optimasi compiler (lihat
`ekstrak_item_gabung_diri()` & `tambahkan_elemen_inplace()` di
`src/lib.rs`, dan `Instr::TambahkanLokal`/`TambahkanGlobal`) yang
mendeteksi pola bytecode PERSIS `x = gabung(x, item)` -- pola paling umum
buat build list di dalam loop -- lalu menggantinya dengan append in-place
lewat `Rc::make_mut` (O(1) amortized kalau list itu tidak sedang di-alias
variabel lain; kalau di-alias, tetap clone seperti biasa -- correctness
immutability TIDAK PERNAH dikorbankan demi kecepatan). Diimplementasikan
di KEDUA jalur compiler (bytecode lama `compile_stmt` DAN jalur IR yang
dipakai `isoteri bangun`/AOT `lower_stmt` -- sempat ketahuan keduanya
terpisah, jadi harus difix dua-duanya supaya AOT benar-benar kebagian
manfaatnya).

**Hasil sesudah perbaikan** (workload `daftar_operasi`, N=20.000 sama
persis, TIDAK diubah supaya perbandingan before/after adil):

| | Sebelum | Sesudah |
|---|---:|---:|
| Isoteri (AOT) | ~4.070ms | **~20ms** |
| vs Node.js | 130x lebih lambat | **1.6x lebih cepat** |
| vs Python | 260x lebih lambat | 1.2x lebih lambat |

**Verifikasi correctness**: 13/13 test regresi lulus (`scripts/regresi.sh`,
termasuk `tes_regresi/gabung_inplace.iso` yang baru ditulis khusus buat
mengunci perilaku ini -- termasuk kasus ALIASING eksplisit: `salinan = b`
lalu `b = gabung(b, item)` -- `salinan` HARUS tetap versi lama, dan
memang terverifikasi tetap benar lewat 3 jalur eksekusi sekaligus, bukan
cuma AOT).

### 2. Konstruksi `Peta` literal + `coba/tangkap` (try/catch) berat per panggilan -- BELUM DIPERBAIKI

Isolasi manual (lihat riwayat kerja sesi ini) menunjukkan: bikin `Peta`
literal 500.000 kali makan ~0.5 detik sendirian; menambah `coba/tangkap` +
3 kali akses indeks Peta menambah ~0.3 detik lagi. V8 (Node.js) dan
CPython punya implementasi dict/exception yang sudah dioptimasi puluhan
tahun (hidden classes/inline caching di V8, dict yang diimplementasi C
native di CPython) -- Isoteri belum kompetitif di sini. Workload
`validasi_petani` MASIH 31x lebih lambat dari Node.js setelah perbaikan
`gabung()` di atas -- karena penyebabnya memang beda, belum tersentuh.

**Implikasi buat roadmap**: ini area optimasi terpisah dari `gabung()` --
kemungkinan representasi `Peta` (saat ini kemungkinan besar masih
`HashMap`/`Vec` generik per instans) dan/atau overhead setup frame
`coba/tangkap` di VM. Ini SEKARANG jadi kandidat optimasi prioritas
tertinggi berikutnya (menyusul `gabung()` yang sudah selesai) -- lebih
penting daripada backend WASM asli (item #5 roadmap), karena membatasi
program apa pun yang validasi/proses banyak record berstruktur, bukan
cuma kasus ekstrem.

### Kenapa `fib_rekursif` menang telak

Tidak ada alokasi objek sama sekali -- murni pemanggilan fungsi & aritmatika
integer. Ini jalur yang paling matang di Isoteri: Cranelift JIT + AOT
native compile mengalahkan V8 (yang tetap punya overhead JIT warm-up +
representasi angka dinamis) dan jauh mengalahkan CPython (interpreter
bytecode murni, tanpa JIT sama sekali secara default).

## Kesimpulan jujur

Klaim "Isoteri lebih cepat di backend" **BENAR UNTUK 2 DARI 3 WORKLOAD**
setelah perbaikan `gabung()`: `fib_rekursif` (CPU murni) dan
`daftar_operasi` (build+map+filter list) sekarang sama cepat atau lebih
cepat dari Node.js. Yang MASIH bersyarat: kode yang berat di alokasi
`Peta`/`coba-tangkap` (`validasi_petani`, notabene contoh utama use-case
Isoteri sendiri!) -- Isoteri AOT di sana masih 31x lebih lambat dari
Node.js. Ini bukan lagi kandidat optimasi yang "mungkin penting" --
angkanya sudah membuktikan ini BLOCKER NYATA buat klaim "lebih cepat di
backend" secara umum, bukan cuma di kasus tertentu.

Ini bukan alasan untuk berhenti -- ini justru PETA JALAN OPTIMASI yang

jelas dan terukur: `gabung()` SUDAH diperbaiki jadi amortized O(1)
(lihat hasil before/after di atas -- ini yang membuktikan pendekatan
"benchmark jujur -> temuan konkret -> perbaikan terukur" ini bekerja).
Berikutnya: selidiki/optimasi biaya konstruksi `Peta` + `coba/tangkap`,
yang sekarang jadi bottleneck TERBESAR yang tersisa. Jalankan ulang
benchmark yang SAMA PERSIS di folder ini untuk lihat progress ke depannya
-- itulah gunanya benchmark ini ada sebagai aset permanen di repo, bukan
cuma laporan sekali pakai.

## Keterbatasan benchmark ini

- Dijalankan di satu mesin, satu sesi -- bukan lintas berbagai hardware
  atau kondisi beban sistem yang bervariasi.
- N tiap workload dipilih supaya total waktu benchmark tetap wajar
  dijalankan berulang saat development (idealnya, iterasi cepat) --
  bukan dioptimasi untuk mensimulasikan beban produksi yang presisi.
- Startup cost Node.js/Python (~30-50ms buat kasus kosong) mendominasi
  workload yang sangat cepat (`daftar_operasi` di Node/Python) --
  perbandingan jadi kurang bermakna di ujung skala ini; workload yang
  lebih besar/lama akan lebih representatif buat mengukur throughput
  murni terpisah dari overhead startup proses.
- Belum ada perbandingan versus WASM asli di browser (item #5 roadmap,
  belum dikerjakan) -- benchmark ini KHUSUS sisi backend/server.
