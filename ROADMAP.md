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
- [x] `putus` / `lanjut` (break/continue) -- aman dipakai di dalam `coba/tangkap`, di eksekusi normal & web export (belum di `via-ir`/AOT, lihat KETERBATASAN.md)
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
- [x] **Component System** (`komponen_buat`/`komponen_pasang`/`komponen_atur_state`/`komponen_lepas`/dst.) -- render-ulang-penuh + event delegation `data-aksi` + lifecycle hooks (`dipasang`/`diperbarui`/`dilepas`)

Lihat `runtime/web/README.md` untuk kemampuan browser yang benar-benar
tersedia saat ini, dan `docs/KETERBATASAN.md` untuk batasan jujur tiap fitur
di atas (termasuk kenapa Component System bukan pengganti vdom-diffing React).

## Prioritas eksplorasi berikutnya

- [ ] Clipboard (copy/paste)
- [ ] History API / path routing (alternatif hash routing yang sudah ada)
- [ ] `dom_ketika()` belum bisa `removeEventListener`
- [ ] Nested/composed components otomatis (sekarang manual lewat placeholder + `komponen_pasang` di hook)
- [ ] HTTP Interceptor -- belum primitif bahasa baru, tapi bisa disusun sendiri di atas `unduh_lanjut_async` (lihat KETERBATASAN.md)
- [ ] Error reporting browser yang lebih baik
- [ ] Dokumentasi pola aplikasi web (tutorial component+router+state end-to-end)
- [ ] Contoh aplikasi web yang lebih lengkap
- [ ] Automated regression test yang lebih luas (formal, bukan cuma skrip manual)
- [ ] `putus`/`lanjut` di jalur `via-ir`/AOT (sekarang panik dengan pesan jelas kalau dicoba, bukan diimplementasikan)
- [ ] Overflow-trapping di JIT (sekarang bytecode VM sudah aman, JIT masih diam-diam wrap -- lihat KETERBATASAN.md)

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

Target WebAssembly asli pernah masuk roadmap, tetapi saat ini ditunda.
Jalur browser yang digunakan sekarang adalah ekspor bundel + VM JavaScript,
dan sudah diperluas jadi kerangka kerja aplikasi web (Router + State +
Component System) di atas jalur itu -- lihat section "Web" di atas.

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

