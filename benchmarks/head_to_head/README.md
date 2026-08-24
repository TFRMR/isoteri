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

| Workload | Pemenang | Margin |
|---|---|---|
| `fib_rekursif` | **Isoteri (AOT)** | 2.6x lebih cepat dari Node.js, 12x dari Python |
| `validasi_petani` | Node.js | Isoteri 30x LEBIH LAMBAT dari Node.js, 6x lebih lambat dari Python |
| `daftar_operasi` | Python | Isoteri 260x LEBIH LAMBAT dari Python, 130x dari Node.js |

**Ini bukan salah ketik.** Isoteri AOT menang telak di komputasi murni
(`fib_rekursif`), tapi kalah jauh di dua workload lain. Ini temuan yang
dilaporkan APA ADANYA sesuai prinsip "klaim lebih cepat yang jujur" di
ROADMAP.md -- bukan cherry-picking hasil yang bagus saja.

## Analisis: kenapa hasilnya timpang begini?

Investigasi cepat (lihat riwayat kerja) mengisolasi penyebabnya jadi dua
karakteristik nyata Isoteri saat ini, BUKAN bug di benchmark:

### 1. `gabung()` (list append) bersifat immutable -- O(n) per panggilan

Didokumentasikan di `docs/REFERENSI.md`: `gabung(daftar, item)`
"kembalikan Daftar BARU dengan item ditambahkan di akhir" -- artinya
tiap panggilan meng-copy seluruh isi list sejauh ini. Build list N elemen
lewat `gabung()` di dalam loop itu O(n) per panggilan / **O(n^2) total**.
Node.js `.push()` dan Python `.append()` itu O(1) amortized (standar
industri). Ini penyebab utama `daftar_operasi` kalah 130-260x meski N-nya
cuma 20.000 (dipilih kecil justru supaya tidak timeout -- dengan N=1 juta
seperti draft awal, Isoteri AOT tidak selesai dalam waktu wajar sama
sekali).

**Implikasi buat roadmap**: `gabung()` versi amortized O(1) (append
sungguhan ke buffer yang sama, bukan copy penuh) adalah kandidat optimasi
BERDAMPAK TINGGI -- lebih penting daripada backend WASM asli (item #5),
karena ini membatasi HAMPIR SEMUA program Isoteri yang memproses data
dalam list, bukan cuma kasus ekstrem.

### 2. Konstruksi `Peta` literal + `coba/tangkap` (try/catch) berat per panggilan

Isolasi manual (lihat riwayat kerja sesi ini) menunjukkan: bikin `Peta`
literal 500.000 kali makan ~0.5 detik sendirian; menambah `coba/tangkap` +
3 kali akses indeks Peta menambah ~0.3 detik lagi. V8 (Node.js) dan
CPython punya implementasi dict/exception yang sudah dioptimasi puluhan
tahun (hidden classes/inline caching di V8, dict yang diimplementasi C
native di CPython) -- Isoteri belum kompetitif di sini.

**Implikasi buat roadmap**: ini area optimasi terpisah dari `gabung()` --
kemungkinan representasi `Peta` (saat ini kemungkinan besar masih
`HashMap`/`Vec` generik per instans) dan/atau overhead setup frame
`coba/tangkap` di VM. Worth diteliti lebih lanjut sebagai item roadmap
baru, TAPI di luar scope item #4 (benchmark) ini -- item ini cuma
melaporkan angka, bukan memperbaikinya.

### Kenapa `fib_rekursif` menang telak

Tidak ada alokasi objek sama sekali -- murni pemanggilan fungsi & aritmatika
integer. Ini jalur yang paling matang di Isoteri: Cranelift JIT + AOT
native compile mengalahkan V8 (yang tetap punya overhead JIT warm-up +
representasi angka dinamis) dan jauh mengalahkan CPython (interpreter
bytecode murni, tanpa JIT sama sekali secara default).

## Kesimpulan jujur

Klaim "Isoteri lebih cepat di backend" **BENAR TAPI BERSYARAT**: cuma
valid untuk kode yang CPU-bound / komputasi-berat tanpa banyak alokasi
list/map. Untuk kode yang berat di alokasi struktur data (kasus umum di
banyak aplikasi web nyata, termasuk `validasi_petani` yang notabene contoh
utama use-case Isoteri sendiri!), Isoteri AOT saat ini KALAH JAUH dari
Node.js maupun Python.

Ini bukan alasan untuk berhenti -- ini justru PETA JALAN OPTIMASI yang
jelas dan terukur: perbaiki `gabung()` jadi amortized O(1), dan
selidiki/optimasi biaya konstruksi `Peta` + `coba/tangkap`. Setelah itu,
jalankan ulang benchmark yang SAMA PERSIS di folder ini untuk lihat
progress -- itulah gunanya benchmark ini ada sebagai aset permanen di
repo, bukan cuma laporan sekali pakai.

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
