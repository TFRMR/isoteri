# Roadmap Isoteri

Roadmap ini adalah peta eksplorasi, bukan janji jadwal. Status dan prioritas
dapat berubah berdasarkan hasil eksperimen dan kontribusi komunitas.

## Sudah ada

### Bahasa & compiler

- [x] Lexer / parser
- [x] AST dan resolver
- [x] Bytecode compiler
- [x] VM
- [x] Tipe dasar
- [x] `bentuk`
- [x] field bersarang
- [x] closure
- [x] modul dengan `muat`
- [x] penanganan error `coba` / `tangkap`
- [x] fungsi teks, matematika, list, JSON, file, dan HTTP
- [x] `lainnya kalau` (else-if) -- gula sintaksis murni, jalan di semua jalur eksekusi
- [x] `putus` / `lanjut` (break/continue) -- aman dipakai di dalam `coba/tangkap`, di SEMUA jalur eksekusi (native, `via-ir`, AOT, web export)
- [x] Modulo (`%`), compound assignment (`+=` dst.), increment/decrement (`++`/`--`)
- [x] Assignment lewat indeks (`daftar[0] = x`, `peta["k"] = x`) -- termasuk nested & campur field, immutable/clone-on-write
- [x] Negasi boolean (`!ekspr`) -- pakai truthiness yang sama dengan `kalau`/`dan`/`atau`
- [x] Closure langsung sebagai callback `petakan`/`saring`/`urutkan` (dulu cuma nama fungsi via Teks)
- [x] Overflow `Angka` terdeteksi jelas di bytecode VM (`checked_add`/`sub`/`mul`), termasuk saat constant-folding compile-time
- [x] Deklarasi ulang `ingat` nama sama sekarang gagal kompilasi (dulu diam-diam menimpa)

### Performa

- [x] JIT dengan Cranelift
- [x] JIT multi-parameter
- [x] JIT untuk `Desimal`
- [x] JIT terbatas untuk struct numerik
- [x] AOT executable native

### Web

- [x] Ekspor program ke bundel web
- [x] Isoteri VM di browser
- [x] DOM dasar
- [x] Storage
- [x] Canvas 2D
- [x] WebSocket dasar
- [x] Event DOM diperluas -- closure & nama Teks, baca data event (`e.nilai`/`e.tombol`/`e.tipe`/`e.target`), backward-compatible dgn handler 0-parameter lama
- [x] Form & input (`dom_nilai`/`dom_atur_nilai`/`dom_dicentang`/`dom_atur_dicentang`/`dom_fokus`)
- [x] Timer browser (`tunda`/`interval_mulai`/`interval_hentikan`)
- [x] Bridge fetch/HTTP diperluas (`unduh_lanjut_async` -- POST/header/body/status code)
- [x] **Router** (`rute_daftar`/`rute_mulai`/`rute_navigasi`/`rute_sekarang`) -- hash-based, path param dinamis (`:id`), catch-all (`*`), query string
- [x] **State Management** (`state_buat`/`state_nilai`/`state_atur`/`state_ubah`/`state_langgan`) -- pola pub/sub sederhana
- [x] **Component System** (`komponen_buat`/`komponen_pasang`/`komponen_atur_state`/`komponen_lepas`/dst.) -- render-ulang-penuh + event delegation `data-aksi` + lifecycle hooks (`dipasang`/`diperbarui`/`dilepas`) + nested/composed components otomatis (`komponen_anak`)

Lihat `runtime/web/README.md` untuk kemampuan browser yang benar-benar
tersedia saat ini, dan `docs/KETERBATASAN.md` untuk batasan jujur tiap fitur
di atas (termasuk kenapa Component System bukan pengganti vdom-diffing React).

## Prioritas eksplorasi berikutnya

- [ ] Clipboard (copy/paste)
- [ ] History API / path routing (alternatif hash routing yang sudah ada)
- [ ] `dom_ketika()` belum bisa `removeEventListener`
- [x] Nested/composed components otomatis (`komponen_anak(komponen, kunci, props)` dipanggil di dalam `render` induk -> runtime otomatis mount/update/unmount anak lewat rekonsiliasi berbasis kunci stabil, rekursif tanpa batas kedalaman, state anak DIPERTAHANKAN lintas render ulang induk -- lihat KETERBATASAN.md)
- [ ] HTTP Interceptor -- belum primitif bahasa baru, tapi bisa disusun sendiri di atas `unduh_lanjut_async` (lihat KETERBATASAN.md)
- [ ] Error reporting browser yang lebih baik
- [ ] Dokumentasi pola aplikasi web (tutorial component+router+state end-to-end)
- [ ] Contoh aplikasi web yang lebih lengkap
- [x] Automated regression test yang lebih luas (`scripts/regresi.sh` + `tes_regresi/` -- bandingkan 3 jalur eksekusi (bytecode murni via `ISOTERI_NO_JIT=1`, JIT produksi, via-ir) satu sama lain DAN terhadap golden file `.out`, dengan allowlist eksplisit `tes_regresi/divergensi_diketahui.txt` buat divergensi yang sudah diverifikasi manual sebagai "beda tapi sama-sama benar". Diverifikasi bisa nangkep regresi sungguhan: bug wrap-around overflow JIT sesi sebelumnya sengaja dimasukkan ulang & langsung ketauan lewat 3 cara sekaligus.)
- [x] `putus`/`lanjut` di jalur `via-ir`/AOT (IrLower sekarang punya loop_stack/LoopCtxIr + coba_depth counter sendiri, pola sama persis dengan Compiler::LoopCtx di bytecode; diverifikasi lewat nested loop & putus/lanjut di dalam coba/tangkap di dalam loop, hasilnya identik dengan jalur biasa)
- [x] Overflow-trapping di JIT (kedua jalur -- `kompilasi()` produksi & `kompilasi_dari_ir()` via-ir/AOT -- sekarang catchable & konsisten dengan bytecode VM, termasuk lewat rekursi dalam; lihat KETERBATASAN.md)

## Eksperimen desain bahasa

- [x] ~~Evaluasi assignment untuk `Daftar` dan `Peta`~~ -- selesai, lihat "Sudah ada" di atas
- [x] ~~Evaluasi `putus` / `lanjut`~~ -- selesai (native+web), lihat "Sudah ada" di atas
- [x] ~~Evaluasi `else-if`~~ -- selesai, lihat "Sudah ada" di atas
- [x] ~~Evaluasi closure pada `petakan` / `saring` / `urutkan`~~ -- selesai, lihat "Sudah ada" di atas
- [ ] Evaluasi namespace modul
- [ ] Evaluasi representasi data numerik yang lebih flat
- [ ] Semver range di package registry (v2) -- v1 git-based/pin-exact-tag sudah selesai

## Eksperimen performa

- [ ] Benchmark VM vs JIT pada workload nyata
- [ ] Benchmark Isoteri vs implementasi pembanding yang relevan
- [ ] Eksperimen representasi `Daftar` numerik
- [ ] Evaluasi SIMD hanya jika representasi data mendukungnya

## WebAssembly

Target WebAssembly asli pernah masuk roadmap, sempat ditunda -- sekarang
**berjalan lagi, scaffold-nya sudah ada**: lihat `isoteri-wasm/` (crate
`wasm-bindgen` tipis, memanggil `isoteri::ekspor_json_dari_sumber()` langsung
-- BUKAN reimplementasi compiler, jadi tidak ada risiko divergensi perilaku).
Sudah divalidasi PENUH secara native (`cargo check`/`build`/`test` semua
lulus, termasuk perbandingan byte-identik dengan hasil CLI `isoteri
ekspor-web` untuk source yang sama). Untuk itu, `isoteri/Cargo.toml` sekarang
punya fitur `jit`/`native-http` (default ON, nol dampak ke CLI biasa) yang
memisahkan Cranelift/`ureq` (gak jalan di wasm32) dari inti compiler bytecode
(yang sudah SEJAK AWAL didesain jalan tanpa JIT sama sekali).

**Build sungguhan ke target `wasm32-unknown-unknown` -- SELESAI & TERVALIDASI**
(build dilakukan di mesin lokal dengan akses internet penuh, bukan di sandbox
kerja yang tidak punya akses ke `static.rust-lang.org`). Langkah `rustup
target add wasm32-unknown-unknown` + `wasm-pack build --target web --out-dir
pkg` dari folder `isoteri-wasm/` berhasil menghasilkan `pkg/isoteri_wasm.js` +
`pkg/isoteri_wasm_bg.wasm`. Diverifikasi end-to-end lewat
`runtime/web/demo_wasm.html` (textarea source `.iso` -> `kompilasi()` WASM ->
`IsoteriVM` dari `isoteri-vm.js`, dilayani via `python3 -m http.server`):
kode contoh (`fungsi`, `kembalikan`, string concat, ekspresi aritmatika)
menghasilkan output yang benar (`"Halo, Dunia!"` dan `42`) langsung di
browser, tanpa CLI sama sekali di jalur ini.

Dengan ini, jalur browser TIDAK LAGI butuh langkah "ekspor bundel lewat CLI"
sebagai satu-satunya cara -- source `.iso` mentah bisa langsung dikompilasi
di browser, membuka jalan buat tool semacam Isoteri AI Studio menghasilkan
satu file HTML utuh yang langsung jalan tanpa compile step terpisah. Jalur
ekspor bundel + VM JavaScript yang sudah ada (Router + State + Component
System, lihat section "Web" di atas) TETAP dipakai persis sama --
`isoteri-wasm` cuma mengganti CARA bundle JSON-nya dihasilkan (di browser,
bukan CLI), bukan mengganti apa yang dijalankan VM-nya.

Belum: `demo_wasm.html` baru menguji subset kecil bahasa (fungsi, string,
aritmatika) -- belum diverifikasi lewat WASM untuk fitur yang lebih kompleks
(struct/`bentuk`, closure, loop, DOM binding penuh). `pkg/` hasil build juga
belum di-commit permanen ke lokasi final di repo/CI (saat ini disalin manual
ke `runtime/web/pkg/` di mesin lokal).

## Prinsip roadmap

Isoteri tidak mengejar "menggantikan semua JavaScript" sebagai tujuan tunggal.
Eksperimen yang lebih penting adalah menemukan:

1. bagian logic aplikasi web yang dapat ditulis nyaman dengan Isoteri,
2. browser API apa yang paling berguna untuk dijembatani,
3. apakah VM/bytecode memberikan keuntungan praktis,
4. bagaimana bahasa domain Indonesia dapat meningkatkan keterbacaan,
5. dan batas nyata Isoteri dibanding stack web biasa.

Salah satu temuan konkret dari eksperimen Component System: `isoteri-vm.js`
TIDAK punya JIT (beda dari native Rust yang punya Cranelift), jadi komputasi
berat (rekursi dalam, dsb) harus tetap dilakukan native/API, bukan langsung
di browser -- diverifikasi langsung (`fib(38)`: <5 detik native, >90 detik
browser). Ini contoh nyata batas real Isoteri-di-browser dibanding
Isoteri-native, dan kenapa arsitektur "compiler sekali, jalankan di banyak
backend" tetap penting: developer bisa pilih backend yang tepat sesuai beban
kerjanya, bukan terjebak satu-satunya pilihan.

Jika hasil eksperimen menunjukkan suatu pendekatan tidak memberi manfaat,
hasil negatif tetap dianggap informasi yang berguna dan sebaiknya
didokumentasikan.

