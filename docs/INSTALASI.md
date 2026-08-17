# Instalasi & Build Isoteri

## Prasyarat

- **Rust & Cargo** — direkomendasikan versi terbaru stabil dari [rustup.rs](https://rustup.rs). Isoteri sendiri dikembangkan & diuji dengan `rustc 1.75.0`, tapi versi lebih baru seharusnya bekerja tanpa masalah.
- Sistem operasi: Linux/macOS (belum diuji di Windows, kemungkinan perlu penyesuaian kecil untuk path file).

## Build dari source

```bash
git clone <url-repo-isoteri>
cd isoteri
cargo build --release
```

Binary hasil build ada di `target/release/isoteri`.

## Menjalankan program

```bash
./target/release/isoteri path/ke/program.iso
```

atau lewat `cargo run`:

```bash
cargo run --release -- path/ke/program.iso
```

---

## ⚠️ Kalau `cargo build` gagal dengan error `edition2024`

Kalau muncul error seperti:

```
error: failed to parse manifest at `.../indexmap-2.x.x/Cargo.toml`
Caused by:
  feature `edition2024` is required
  The package requires the Cargo feature called `edition2024`, but that
  feature is not stabilized in this version of Cargo.
```

Ini artinya `rustc`/`cargo` yang kamu pakai **lebih lama** dari yang dibutuhkan versi terbaru dependency transitif (`indexmap`, `idna_adapter`, `zeroize`, dkk — datang dari `ureq` → `url` → `idna` dan dari `cranelift-jit` → `wasmtime-jit-icache-coherence`). Ini bukan bug di kode Isoteri sendiri, tapi bentrokan versi toolchain vs dependency.

**Solusi paling gampang: update Rust ke versi terbaru.**

```bash
rustup update stable
```

**Kalau update Rust bukan opsi** (misal di environment terbatas/sandbox tanpa akses internet ke `rust-lang.org`), pin dependency yang bermasalah ke versi lebih lama yang kompatibel:

```bash
cargo update -p indexmap --precise 2.2.6
cargo update -p idna_adapter --precise 1.1.0
cargo update -p zeroize --precise 1.8.1
cargo build --release
```

Kombinasi versi di atas **sudah terverifikasi bekerja dengan `rustc 1.75.0`**. Kalau masih muncul error `edition2024` untuk paket lain setelah pin ini (dependency Rust terus berkembang, jadi kombinasi versi kompatibel bisa berubah seiring waktu), pola perbaikannya sama: baca nama paket dari pesan error, cari versi rilis lebih lama di halaman [crates.io](https://crates.io) milik paket itu (biasanya beberapa minor version ke belakang cukup), lalu `cargo update -p <nama_paket> --precise <versi>`.

---

## Menjalankan test suite / contoh program

Semua file `.iso` di root repo (`program*.iso`) dan folder `modul_test/` adalah contoh program yang berfungsi ganda sebagai regression test manual. Jalankan satu per satu dan bandingkan output-nya (tidak ada automated test harness saat ini — lihat [KETERBATASAN.md](KETERBATASAN.md)):

```bash
for f in program*.iso; do
  echo "=== $f ==="
  ./target/release/isoteri "$f"
done
```

## Dependency yang dipakai

| Crate | Kegunaan |
|---|---|
| `ureq` | HTTP client, untuk fungsi bawaan `unduh()` |
| `cranelift`, `cranelift-jit`, `cranelift-module`, `cranelift-native` | Backend kompilasi JIT ke kode mesin asli |

Tidak ada dependency lain di luar yang tercantum di `Cargo.toml` — parser JSON, lexer, VM bytecode, dan semuanya ditulis manual tanpa crate eksternal (zero-dependency by design untuk bagian inti bahasa).
