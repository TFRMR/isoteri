# isoteri-wasm

Wrapper `wasm-bindgen` **tipis** di atas compiler Isoteri asli. Satu-satunya
isi berarti: memanggil `isoteri::ekspor_json_dari_sumber()` langsung -- fungsi
PERSIS yang sama dipakai CLI native (`isoteri ekspor-web`). Sudah diverifikasi
manual byte-identik dengan hasil CLI untuk source yang sama.

Begitu crate ini di-build jadi `.wasm`, Isoteri AI Studio (dan halaman web apa
pun) bisa kompilasi source Isoteri **langsung di browser** -- tanpa CLI, tanpa
langkah compile terpisah. Ini yang bikin alur "AI Studio -> HTML jadi, langsung
jalan" mungkin (dibanding alur 2-langkah sekarang: generate `.iso` -> compile
CLI -> HTML jalan).

## Kenapa gak bisa di-build di sesi/environment ini

Environment kerja (sandbox) tidak punya akses network ke `static.rust-lang.org`
(cuma `crates.io` dkk yang di-allowlist), jadi `rustup target add
wasm32-unknown-unknown` -- yang butuh unduh precompiled `std`/`core` buat
target itu -- tidak bisa jalan dari sana. Kode di `src/lib.rs` sudah divalidasi
penuh secara **native** (`cargo check`, `cargo build`, `cargo test` semua
sukses, termasuk perbandingan byte-identik vs CLI) -- yang belum tervalidasi
cuma langkah kompilasi ke target `wasm32` itu sendiri, yang butuh mesin dengan
akses internet penuh (mesinmu).

## Cara build (di mesin dengan akses internet penuh)

```bash
# 1. Sekali saja: pasang rustup (kalau belum ada) & target wasm32
rustup target add wasm32-unknown-unknown

# 2. Sekali saja: pasang wasm-pack (tool resmi buat build crate Rust -> paket JS+wasm)
cargo install wasm-pack

# 3. Build (dari folder isoteri-wasm/ ini)
wasm-pack build --target web --out-dir pkg

# Hasilnya di isoteri-wasm/pkg/:
#   isoteri_wasm.js       -- glue JS, di-import dari halaman
#   isoteri_wasm_bg.wasm  -- modul wasm-nya
#   isoteri_wasm.d.ts     -- (opsional) type definitions kalau perlu
```

Kalau `cargo check`/`cargo build` (native, BUKAN wasm32) di repo ini sempat
gagal duluan gara-gara `wasm-bindgen-shared` butuh rustc lebih baru dari yang
terpasang: `Cargo.toml` di sini sudah dikunci ke `wasm-bindgen = "=0.2.92"`
(versi yang jalan di rustc 1.75 -- versi apt Ubuntu 24.04 default) supaya build
di environment manapun tetap konsisten. Kalau mesinmu sudah pakai rustc lebih
baru (disarankan, lewat rustup, BUKAN apt), boleh naikkan versinya:
`cargo add wasm-bindgen@^0.2` dari folder ini.

## Cara pakai hasilnya (dari HTML/JS)

```html
<script type="module">
  import init, { kompilasi, versi } from "./pkg/isoteri_wasm.js";

  await init(); // muat modul .wasm sekali di awal

  console.log("Compiler Isoteri (wasm):", versi());

  const sumberIsoteri = `
    fungsi sapa(nama) { kembalikan "Halo, " + nama }
    tampilkan sapa("Dunia")
  `;

  try {
    const bundelJson = kompilasi(sumberIsoteri);       // Teks JSON, SAMA PERSIS format .isoweb.json
    const bundel = JSON.parse(bundelJson);
    const vm = new IsoteriVM(bundel);                   // dari isoteri-vm.js, TANPA perubahan apa pun
    vm.jalankan();
  } catch (pesanError) {
    console.error("Gagal kompilasi:", pesanError);       // Teks error Isoteri asli (Lexer/Parser/Kompilasi)
  }
</script>
```

Begitu `pkg/` ini ada & isoteri-vm.js dimuat, halaman HTML bisa memuat
**langsung dari source `.iso` mentah** -- tidak perlu `.isoweb.json` terpisah
lagi. Ini yang membuka jalan buat Isoteri AI Studio (lihat folder
`isoteri-studio/` di luar repo compiler ini) menghasilkan SATU file HTML utuh
yang langsung jalan tanpa langkah compile CLI terpisah sama sekali.

## Yang TIDAK berubah/berkurang

Crate `isoteri` utama (native CLI) **tidak terpengaruh sama sekali** oleh
keberadaan crate ini -- `cargo build --release` dari folder utama tetap
menghasilkan binary CLI persis seperti sebelumnya (JIT aktif, `unduh()` asli,
dst.). Fitur `jit`/`native-http` yang ditambahkan di `isoteri/Cargo.toml`
default-nya ON, cuma isoteri-wasm/ ini yang secara eksplisit mematikannya
lewat `default-features = false` (lihat komentar di `Cargo.toml` folder ini).
