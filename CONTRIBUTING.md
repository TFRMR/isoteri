# Berkontribusi ke Isoteri

Terima kasih sudah tertarik mengembangkan Isoteri.

Isoteri saat ini adalah proyek bahasa pemrograman dan runtime yang masih
eksperimental. Kontribusi tidak harus berupa perubahan compiler besar.
Contoh program, dokumentasi, test, benchmark, browser runtime, dan laporan
bug semuanya sangat berguna.

## Jenis kontribusi

- **Bahasa** — lexer, parser, resolver, compiler, VM, JIT, dan semantic checks.
- **Runtime Web** — DOM, Canvas, WebSocket, dan bridge API browser lainnya.
- **Dokumentasi** — tutorial, referensi, contoh, dan penjelasan error.
- **Test & benchmark** — regresi, kasus batas, dan pengukuran performa.
- **Contoh** — program `.iso` kecil yang menunjukkan kemampuan bahasa.
- **Issue** — bug report, usulan fitur, atau pertanyaan desain.

## Sebelum membuat Pull Request

Dari root repository:

```bash
cargo build --release
```

Jalankan contoh/regresi yang relevan dengan perubahanmu. Untuk perubahan
bahasa atau runtime, sertakan contoh minimal atau test yang menunjukkan
perilaku baru.

Jika perubahan menyentuh browser runtime, uji juga contoh di:

```text
runtime/web/
```

## Alur sederhana

```text
Fork
  ↓
Buat branch
  ↓
Buat perubahan
  ↓
Build + test
  ↓
Commit
  ↓
Pull Request
```

Contoh:

```bash
git checkout -b fitur/nama-fitur
git add .
git commit -m "feat: jelaskan perubahan"
git push origin fitur/nama-fitur
```

Lalu buka Pull Request di GitHub.

## Prinsip pengembangan

1. Utamakan perubahan kecil dan mudah ditinjau.
2. Jangan menyembunyikan keterbatasan yang diketahui.
3. Jika menambah kemampuan bahasa, dokumentasikan sintaks dan semantiknya.
4. Jika memperbaiki bug, tambahkan contoh regresi bila memungkinkan.
5. Untuk perubahan performa, sertakan benchmark bila relevan.
6. Jangan memasukkan secret, token, password, atau kredensial ke repository.

## Status eksperimental

Isoteri belum ditujukan sebagai pengganti JavaScript secara penuh dan belum
dinyatakan production-ready. API, sintaks, dan arsitektur dapat berubah.

Sebelum mengusulkan fitur besar, baca:

- `README.md`
- `docs/FILOSOFI.md`
- `docs/KETERBATASAN.md`
- `docs/REFERENSI.md`

## Ide yang belum matang juga boleh

Untuk proyek eksplorasi bahasa, diskusi desain sangat berharga. Jika sebuah
ide belum siap menjadi kode, buka Issue terlebih dahulu dan jelaskan:

- masalah yang ingin diselesaikan,
- contoh sintaks yang diusulkan,
- perilaku yang diharapkan,
- alternatif yang sudah dipertimbangkan.
