# Isoteri

Bahasa pemrograman berbahasa Indonesia, ditulis dari nol dalam Rust. Pipeline lengkap: lexer → parser → AST → resolver (slot-based) → bytecode compiler → VM, dengan JIT opsional (via [Cranelift](https://cranelift.dev)) untuk fungsi numerik murni.

```
bentuk Petani {
    nama: Teks,
    lahan_hektar: Angka,
    hasil_panen: Desimal
}

fungsi ambil_hasil(p) { kembalikan p.hasil_panen }

ingat semua_petani = [
    Petani { nama: "Budi", lahan_hektar: 1, hasil_panen: 200.0 },
    Petani { nama: "Siti", lahan_hektar: 2, hasil_panen: 360.0 }
]

ulang setiap p dari urutkan(semua_petani, "ambil_hasil") {
    tampilkan p.nama + ": " + p.hasil_panen + " kg"
}
```

## Mulai Cepat

```bash
cargo build --release
./target/release/isoteri program.iso
```

Kalau build gagal karena error `edition2024`, itu masalah versi toolchain, bukan bug di kode — lihat [docs/INSTALASI.md](docs/INSTALASI.md).

## Dokumentasi

| Dokumen | Isinya |
|---|---|
| [docs/TUTORIAL.md](docs/TUTORIAL.md) | Belajar dari nol, langkah demi langkah, buat yang baru pertama kali pakai Isoteri |
| [docs/REFERENSI.md](docs/REFERENSI.md) | Rujukan lengkap: semua sintaks, operator, dan fungsi bawaan |
| [docs/KETERBATASAN.md](docs/KETERBATASAN.md) | Batasan yang sudah diketahui — baca ini sebelum lapor "bug" |
| [docs/ERROR.md](docs/ERROR.md) | Arti tiap jenis pesan error & cara memperbaikinya |
| [docs/INSTALASI.md](docs/INSTALASI.md) | Cara build dari source, termasuk workaround masalah versi dependency |

## Fitur Bahasa

- Tipe dasar: `Angka` (i64), `Desimal` (f64), `Teks`, `Bool`, `Daftar`, `Peta`, `kosong`
- `bentuk` — struct/tipe custom dengan validasi field saat kompilasi, mendukung field bersarang (baca & tulis, sedalam apapun)
- Closure (fungsi anonim) dengan capture-by-value, termasuk closure bersarang dengan capture transitif
- `muat` — sistem modul antar file, dengan deteksi tabrakan nama lintas-modul
- Fungsi bawaan untuk teks, matematika, list (`petakan`/`saring`/`urutkan`), JSON, file, dan HTTP
- `coba/tangkap` untuk penanganan error runtime
- `ulang selaras` — perulangan paralel multi-thread
- Kompilasi JIT otomatis (ke kode mesin asli lewat Cranelift) untuk fungsi numerik murni — tidak perlu anotasi manual, terdeteksi otomatis
- Kompilasi AOT (`isoteri bangun program.iso -o keluaran`) — hasilkan executable native mandiri, bisa didistribusikan tanpa perlu instalasi apa pun

Detail lengkap tiap fitur dan batasannya ada di [docs/REFERENSI.md](docs/REFERENSI.md) dan [docs/KETERBATASAN.md](docs/KETERBATASAN.md).

## Roadmap

Isoteri dikembangkan lewat dua kelompok prioritas: **Kelompok 1** (kelengkapan dasar bahasa, biar setara Python/JS/Java) dan **Kelompok 2** (fitur yang memaksimalkan keunggulan Rust — performa native yang tidak bisa ditandingi bahasa dinamis biasa).

### Kelompok 1 — Kelengkapan Dasar

| Fitur | Status |
|---|---|
| Struct/tipe custom (`bentuk`) | ✅ Selesai |
| Fungsi string & matematika | ✅ Selesai |
| Fungsi list lanjutan (`urutkan`/`saring`/`petakan`) | ✅ Selesai |
| Modul/import antar file (`muat`) | ✅ Selesai |
| Fungsi anonim/closure | ✅ Selesai |

### Kelompok 2 — Keunggulan Rust

| Fitur | Status |
|---|---|
| JIT multi-parameter | ✅ Selesai |
| JIT untuk tipe Desimal | ✅ Selesai |
| Kompilasi ke executable native mandiri (AOT) | ✅ Selesai |
| Struct yang JIT-able (parameter bentuk numerik-murni di-flatten jadi slot langsung) | ✅ Selesai (versi terbatas — baca, bukan return; lihat batasan di REFERENSI.md) |
| SIMD untuk loop data numerik | ⚠️ Dicoba, TIDAK dilanjutkan — lihat penjelasan di bawah tabel |
| Target WebAssembly asli | ⏸️ Ditunda — butuh target `wasm32-unknown-unknown` (lihat [INSTALASI.md](docs/INSTALASI.md)) |
| **Browser Native (Fase 3 blueprint)** | ✅ **Selesai lewat jalur pragmatis**: `isoteri ekspor-web` + VM JavaScript — lihat [runtime/web/README.md](runtime/web/README.md) |

**Soal SIMD**: sempat diimplementasikan (AVX2, buat `jumlah()`/`rata_rata()`), tapi **dibenchmark langsung dan ternyata lebih lambat** dari versi scalar biasa (~45% lebih lambat di uji coba nyata), bukan lebih cepat — jadi diputuskan **direvert**, bukan diship. Penyebabnya: representasi nilai di Isoteri (`Value` enum, tagged/boxed) bikin data numerik di dalam `Daftar` gak tersimpan sebagai larik mentah yang bisa langsung diproses SIMD — perlu langkah "ekstraksi" ke buffer sementara dulu, dan biaya ekstraksi itu (yang harus tetap jalan elemen-per-elemen) sama besarnya dengan biaya loop scalar biasa. Jadi SIMD-nya nambah kerjaan, bukan gantiin kerjaan. Supaya SIMD beneran menang, `Daftar` numerik-murni butuh representasi memori flat tersendiri (mirip proyek "struct yang JIT-able" di atas, tapi buat list) — itu perubahan arsitektur lebih besar yang belum dikerjakan.

Empat item terakhir Kelompok 2 sengaja ditunda: semuanya optimasi performa yang baru bernilai kalau sudah ada program Isoteri nyata yang cukup besar/berat untuk membutuhkannya. Prioritas saat ini setelah Kelompok 1 selesai adalah membereskan technical debt dan dokumentasi (lihat di bawah) sebelum menambah fitur baru lagi.

### Technical Debt (Selesai Dibereskan)

Selama pengembangan Kelompok 1, ditemukan beberapa keterbatasan arsitektur yang sudah diperbaiki:

- ✅ Fungsi & closure sekarang bisa membaca DAN menulis variabel global (sebelumnya cuma bisa akses parameter & variabel lokal sendiri)
- ✅ Tabrakan nama fungsi/`bentuk`/variabel global antar modul (`muat`) sekarang terdeteksi & gagal kompilasi dengan pesan jelas (sebelumnya diam-diam saling menimpa)
- ✅ Closure level atas sekarang bisa rekursi ke dirinya sendiri lewat namanya
- ✅ Beberapa celah validasi compile-time dibereskan: deklarasi fungsi ganda, parameter nama ganda, field `bentuk` ganda (baik di definisi maupun di literal konstruksi) — semuanya dulu diam-diam diterima, sekarang gagal kompilasi dengan pesan jelas

Batasan yang **masih ada** (bukan bug, tapi keterbatasan desain yang diketahui) didokumentasikan lengkap di [docs/KETERBATASAN.md](docs/KETERBATASAN.md).

### Fase 2 — IR & Optimizer (Milestone A, sedang berjalan)

`CStmt`/`CExpr` diformalkan sebagai **Isoteri IR v1** — satu representasi yang dibaca bersama oleh backend Bytecode, JIT, dan Web (bukan masing-masing menelusuri AST sendiri). Optimizer IR (constant folding + dead code elimination) jalan sekali di sini dan otomatis menguntungkan ketiga backend sekaligus.

Di atasnya ada **IR Linear/Typed (v2)** — representasi tiga-alamat dengan register virtual, langkah menuju `AST -> IR -> {Bytecode, JIT, AOT}` yang sesungguhnya. Divalidasi ketat lewat `isoteri via-ir program.iso` (jalur alternatif yang menjalankan program lewat IR linear, dibandingkan byte-per-byte terhadap jalur produksi):

```bash
isoteri via-ir program.iso        # jalankan lewat IR linear (validasi)
diff <(isoteri program.iso) <(isoteri via-ir program.iso)   # harus kosong
benchmarks/jalankan.sh            # benchmark suite dasar (fib rekursif, loop, list+petakan)
```

**17/17 program contoh cocok persis** di antara kedua jalur. Register allocation v1 sudah mengurangi overhead loop-dalam-fungsi dari ~2x jadi ~15%; stack scheduling (percobaan pertama TERBUKTI SALAH — ditemukan lewat kasus rekursif, diperbaiki jadi versi konservatif) mengurangi sedikit overhead kode global (~2x → ~1.8x, belum tuntas). **Migrasi JIT ke IR linear** juga selesai — Cranelift generate kode mesin langsung dari IR yang sama dipakai backend bytecode, performanya **setara** JIT produksi (laporan "~35% overhead" sebelumnya salah — ternyata benchmark-nya diam-diam tidak lolos JIT sama sekali karena parameter tanpa anotasi tipe; sudah diperbaiki & diverifikasi ulang lewat dump CLIF). **AOT langsung dari IR** juga selesai — `isoteri bangun` sekarang generate binary yang jalan lewat bytecode+JIT dari IR yang sama, diverifikasi 17/17 + performa setara AOT lama. Lihat [docs/IR.md](docs/IR.md) untuk cerita lengkap bug (dan kesalahan benchmark) yang ditemukan.

### Fase 3 — Browser Native

```bash
isoteri ekspor-web program.iso -o program.isoweb.json   # kompilasi ke bytecode JSON
node runtime/web/jalankan-node.js program.isoweb.json    # jalankan di Node.js
# atau buka runtime/web/demo.html untuk jalankan di browser
```

Bytecode Isoteri diekspor ke JSON lalu dijalankan oleh `isoteri-vm.js`, sebuah VM
tulis-ulang di JavaScript yang semantiknya mengikuti persis VM Rust — **diverifikasi
identik byte-per-byte** untuk 13/16 program contoh (sisanya sengaja belum didukung:
`unduh`/`baca_berkas`/`tulis_berkas` dan `ulang selaras`, lihat
[runtime/web/README.md](runtime/web/README.md) untuk detail & alasan pendekatan ini
dibanding menunggu target WASM asli). Lihat juga [docs/FILOSOFI.md](docs/FILOSOFI.md)
untuk 10 Hukum Isoteri dan peta jalan fase lengkap.

### Milestone B — DOM/Event/Storage/Fetch/Canvas/WebSocket

```isoteri
ingat judul = dom_pilih("#judul")
dom_atur_teks(judul, "Halo Isoteri")
dom_ketika(judul, "klik", "ketika_diklik")
simpan_lokal("kunci", "nilai")
unduh_async("https://api.contoh.com", "saat_sukses", "saat_gagal")

ingat ctx = dom_konteks_2d(dom_pilih("#papan"))
kanvas_isi_gaya(ctx, "merah")
kanvas_isi_persegi(ctx, 10, 10, 100, 50)

ingat soket = ws_buka("wss://contoh.com/socket")
ws_ketika_pesan(soket, "saat_pesan")
ws_kirim(soket, "halo")
```

Fungsi bawaan datar (bukan sintaks `objek.metode()` — parser belum mendukungnya,
lihat catatan di [runtime/web/README.md](runtime/web/README.md)), diimplementasikan
sepenuhnya di `runtime/web/isoteri-vm.js` tanpa perubahan compiler Rust sama sekali —
konsisten dengan prinsip "DOM adalah lapisan platform, bukan core language" di
[docs/FILOSOFI.md](docs/FILOSOFI.md). Contoh lengkap: `runtime/web/contoh_dom.iso`
dan `runtime/web/contoh_kanvas_ws.iso`.

### Milestone C — Package Manager Minimal

```bash
isoteri init aplikasi_saya          # bikin isoteri.toml + src/main.iso
isoteri tambah matematika ../lib_matematika          # dependensi lokal
isoteri tambah warna --git https://github.com/x/warna --tag v1.0.0  # registry (git-based)
isoteri                              # jalan (default ke src/main.iso kalau ada isoteri.toml)
isoteri uji                          # jalankan tiap .iso di tes/, exit code nonzero kalau ada yang gagal
```

```isoteri
catatan: di src/main.iso, setelah isoteri tambah:
muat "matematika"
tampilkan kuadratkan(6)
```

`muat "nama_paket"` (tanpa `/` atau akhiran `.iso`) diresolusi otomatis lewat
`isoteri.toml` — dicari ke atas dari direktori berkas saat ini (seperti Cargo
mencari `Cargo.toml`), lalu ke `<path>/src/lib.iso`, baik dependensi lokal
maupun git. Berlaku konsisten di SEMUA jalur: jalan langsung, `isoteri bangun`
(AOT), dan `isoteri ekspor-web`. `gagal_uji("pesan")` untuk assertion di
dalam kasus uji. Manifest di-parse pakai parser tulisan tangan (bukan
dependensi crate `toml`), konsisten dengan gaya proyek ini. Contoh proyek
dua-paket lengkap (aplikasi + dependensi lokal + kasus uji): `contoh_paket/`.

**Registry (v1, git-based)**: `isoteri tambah nama --git URL --tag vX.Y.Z`
(atau `--rev <commit_hash>`, tidak boleh keduanya) mengambil paket lewat
`git clone` ke cache lokal `~/.isoteri/cache/` (override lewat env
`ISOTERI_CACHE_DIR`) — mirip Go modules/Deno, BUKAN indeks server terpusat
kayak npm/Cargo. Sekali tag/rev ke-cache tidak di-fetch ulang (cache = pin).
Butuh `git` terinstal & ada di PATH. Belum ada: version range/semver
resolution (cuma pin exact tag/rev), index/discovery server buat pencarian
paket. Lihat [docs/FILOSOFI.md](docs/FILOSOFI.md) untuk status lengkap &
kerja lanjutan (LSP, formatter).

### `isoteri format` — sumber kebenaran gaya penulisan

```bash
isoteri format program.iso              # rapikan di tempat
isoteri format program.iso --cek        # mode CI: exit nonzero kalau belum rapi, tidak menulis apa pun
```

Cetak ulang dari AST (bukan normalisasi teks apa adanya) — indentasi 4 spasi,
kurung minimal tapi benar secara presedensi, satu gaya konsisten terlepas
dari bagaimana kode aslinya ditulis. Komentar (`catatan: ...`) dipertahankan
lewat jalur terpisah (`Lexer::tokenize_dengan_komentar`) yang **tidak
mengubah Lexer/Parser produksi sedikit pun** — nol risiko regresi ke
compiler. **17/17 program contoh** di proyek ini sudah diformat ulang
(dogfooding) dan diverifikasi *semantically identical* + idempoten. Lihat
[docs/FILOSOFI.md](docs/FILOSOFI.md) untuk cerita bug yang ditemukan
(trailing comma di `bentuk`, tidak didukung Isoteri) dan keterbatasan v1.

## Struktur Proyek

```
src/main.rs          Seluruh implementasi: lexer, parser, resolver, compiler, VM, JIT
Cargo.toml            Dependency: ureq (HTTP), cranelift* (JIT)
program*.iso           Contoh program, berfungsi ganda sebagai regression test manual
modul_test/            Contoh proyek multi-file pakai 'muat'
docs/                  Dokumentasi (lihat tabel di atas)
```
