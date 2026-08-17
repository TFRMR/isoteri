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

Lihat `runtime/web/README.md` untuk kemampuan browser yang benar-benar
tersedia saat ini.

## Prioritas eksplorasi berikutnya

- [ ] Memperluas event DOM
- [ ] Memperluas bridge `fetch` / HTTP di browser
- [ ] Timer browser
- [ ] Form dan input
- [ ] Clipboard
- [ ] Error reporting browser yang lebih baik
- [ ] Dokumentasi pola aplikasi web
- [ ] Contoh aplikasi web yang lebih lengkap
- [ ] Automated regression test yang lebih luas

## Eksperimen desain bahasa

- [ ] Evaluasi assignment untuk `Daftar` dan `Peta`
- [ ] Evaluasi `putus` / `lanjut`
- [ ] Evaluasi `else-if`
- [ ] Evaluasi closure pada `petakan` / `saring` / `urutkan`
- [ ] Evaluasi namespace modul
- [ ] Evaluasi representasi data numerik yang lebih flat

## Eksperimen performa

- [ ] Benchmark VM vs JIT pada workload nyata
- [ ] Benchmark Isoteri vs implementasi pembanding yang relevan
- [ ] Eksperimen representasi `Daftar` numerik
- [ ] Evaluasi SIMD hanya jika representasi data mendukungnya

## WebAssembly

Target WebAssembly asli pernah masuk roadmap, tetapi saat ini ditunda.
Jalur browser yang digunakan sekarang adalah ekspor bundel + VM JavaScript.

## Prinsip roadmap

Isoteri tidak mengejar "menggantikan semua JavaScript" sebagai tujuan tunggal.
Eksperimen yang lebih penting adalah menemukan:

1. bagian logic aplikasi web yang dapat ditulis nyaman dengan Isoteri,
2. browser API apa yang paling berguna untuk dijembatani,
3. apakah VM/bytecode memberikan keuntungan praktis,
4. bagaimana bahasa domain Indonesia dapat meningkatkan keterbacaan,
5. dan batas nyata Isoteri dibanding stack web biasa.

Jika hasil eksperimen menunjukkan suatu pendekatan tidak memberi manfaat,
hasil negatif tetap dianggap informasi yang berguna dan sebaiknya
didokumentasikan.
