# Checklist Sebelum Publish ke GitHub

## Struktur proyek

- [x] `src/`
- [x] `runtime/web/`
- [x] `docs/`
- [x] `benchmarks/`
- [x] `modul_test/`
- [x] `contoh_paket/`
- [x] `Cargo.toml`
- [x] `Cargo.lock`
- [x] `README.md`

## Open-source

- [x] `LICENSE`
- [x] `CONTRIBUTING.md`
- [x] `ROADMAP.md`
- [x] `.gitignore`

## Dokumentasi

- [x] Instalasi
- [x] Tutorial
- [x] Referensi
- [x] Filosofi
- [x] IR
- [x] Error
- [x] Keterbatasan
- [x] Browser runtime README

## Sebelum `git push`

Jalankan:

```bash
cargo build --release
git status
```

Pastikan tidak ada:

- password
- API key
- token
- `.env`
- file pribadi
- `target/`
- build output yang tidak diperlukan

Lalu:

```bash
git add .
git commit -m "chore: prepare Isoteri for open source"
git push
```

## Setelah repository GitHub dibuat

README sebaiknya menampilkan:

- nama proyek,
- status eksperimental,
- contoh kode singkat,
- cara instalasi,
- link dokumentasi,
- link browser demo jika sudah tersedia.

## Catatan

Checklist ini menilai kesiapan struktur dan dokumentasi, bukan menyatakan
bahwa seluruh fitur Isoteri sudah production-ready.
