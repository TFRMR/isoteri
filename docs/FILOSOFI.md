# Filosofi Isoteri

## Identitas
"Isoteri" (dari *esoteric*) diarahkan BUKAN ke makna "bahasa rahasia yang sulit",
tapi ke: **pengetahuan mendalam tentang mesin komputer yang dibuat sederhana
bagi manusia.**

> Deep computing, made human.
> Simple syntax. Native speed. Universal reach.
> Kesederhanaan manusia, kekuatan mesin.

Posisi Isoteri bukan "Rust yang mudah" atau "JavaScript yang cepat", tapi ruang
di antara keduanya: high-level simplicity bertemu low-level performance.

## 10 Hukum Isoteri (jangan dilanggar tanpa alasan kuat)

1. **Simplicity over cleverness** — fitur baru harus lolos uji "apakah programmer
   biasa terbantu?", bukan "apakah ini keren?".
2. **Zero-cost philosophy** — abstraksi (`daftar.petakan()`, dst) harus bisa
   di-compile jadi setara loop biasa, bukan overhead tersembunyi.
3. **One language, multiple worlds** — bukan "Isoteri Web" vs "Isoteri Server"
   yang terpisah, tapi satu bahasa, banyak target compile.
4. **Compiler is the intelligence** — kompleksitas (type inference, optimasi,
   SIMD, paralel) ditanggung compiler, bukan dibebankan ke sintaks pengguna.
5. **Runtime kecil** — target < beberapa MB, bukan runtime raksasa ala Node+V8.
6. **Explicit escape hatch** — selalu ada jalan dari mode mudah ke mode kontrol
   penuh (`miliki`/ownership, `akses_memory()`, dst), tidak dipaksa satu jalan.
7. **Stable core** — bahasa inti berubah lambat & backward-compatible; jangan
   ubah total sintaks di tiap versi mayor.
8. **Jangan** coba jadi "semua fitur Rust" — ambil filosofinya, bukan seluruh
   kompleksitasnya (Rust punya 10+ tahun evolusi & tim besar, kita tidak).
9. **Jangan** coba jadi "JavaScript baru" — tidak akan menang lawan Google/
   Mozilla/Microsoft di ranah itu.
10. **Bahasa Indonesia adalah keunikan, bukan batas** — pintu masuk yang khas,
    bukan alasan untuk membatasi jangkauan/pasar bahasa ini.

## Arsitektur berlapis (biar tidak jadi monster dalam 5 tahun)

```
Layer 1 — Language Core     : variable, function, type, struct, enum, pattern,
                               module, error. HARUS tetap kecil.
Layer 2 — Standard Library  : collections, string, math, io, async
Layer 3 — Platform Library  : web, windows, linux, mobile (terpisah dari core)
Layer 4 — Compiler Backend  : Frontend -> Isoteri IR -> {LLVM | Cranelift |
                               WASM | Interpreter} backend, tidak diikat ke
                               satu teknologi backend saja.
```

## Roadmap fase (dari blueprint asli, dipertahankan sebagai peta jalan)

- **Fase 0/1 — Foundation & Core**: lexer/parser/AST/VM/error/module stabil,
  type system, package manager, formatter, testing. -> sebagian besar SELESAI
  (lihat README.md & KETERBATASAN.md untuk status detail).
- **Fase 2 — Native Performance**: IR, optimizer, native compiler (Cranelift
  JIT sudah ada untuk fungsi numerik murni), SIMD, profiling.
- **Fase 3 — Web Dominance**: WASM target, DOM binding, Browser API, WebGPU,
  `<script type=isoteri>`. -> **prototype kuat sudah jadi** lewat jalur
  pragmatis: ekspor bytecode ke JSON + VM tulis-ulang di JavaScript (lihat
  `runtime/web/`), karena target `wasm32-unknown-unknown` butuh komponen
  rustup yang belum tentu tersedia di semua environment build. Ini bukan
  pengganti permanen WASM asli — begitu toolchain wasm32 tersedia, jalur
  "compile Isoteri IR -> WASM asli" tetap jalur jangka panjang yang benar
  (lebih cepat dari interpreter JS), tapi bytecode-JSON+VM-JS ini sudah
  memenuhi janji "Browser Native" hari ini tanpa menunggu itu. **DOM/Event/
  Storage/Fetch (Milestone B) sudah ada** — lihat `runtime/web/README.md`.
- **Fase 4 — Systems Capability**: ownership mode eksplisit, unsafe mode,
  embedded, driver, game engine.
- **Fase 5 — Ecosystem**: package registry, IDE, community, framework.

## Reprioritisasi (setelah Core/Native/Web ketiganya jadi): IR dulu

Setelah Core, Native (JIT/AOT), dan Web (runtime browser) sama-sama mencapai
titik matang/prototype-kuat, urutan berikutnya diubah dari rencana awal
"package manager → DOM binding" menjadi **membangun lapisan IR dulu**, karena
hampir semua pekerjaan besar berikutnya (SIMD yang benar, migrasi JIT, AOT
langsung dari IR, WASM asli, GPU) sama-sama butuh IR yang solid sebagai
persimpangan jalan — tanpa itu, tiap fitur baru jadi "special case" per
backend yang makin mahal dirawat 2-3 tahun ke depan. Detail lengkap & status
implementasi IR v1 (constant folding + dead code elimination di atas
CStmt/CExpr yang diformalkan jadi IR) ada di **docs/IR.md**.

### Tiga milestone besar (menggantikan checklist datar)

**Milestone A — "Isoteri Compiler"** (prioritas #1, **SEBAGIAN BESAR SELESAI**)
```
AST -> IR -> {Bytecode, JIT, AOT}   + typed IR + optimizer + benchmark suite + regression test
```
Status: IR v1 (constant folding + dead code elimination) sudah jalan di
`src/lib.rs` bagian "4b", dipakai bersama oleh backend Bytecode/JIT/Web
sekaligus. **IR linear/typed (v2)** — representasi tiga-alamat dengan
register virtual — juga sudah jadi (bagian "8b"), divalidasi 17/17 program
contoh cocok byte-per-byte lewat `isoteri via-ir program.iso`. **Register
allocation** (destination-passing + stack scheduling konservatif) sudah
mengurangi overhead loop-dalam-fungsi dari ~2x ke ~15% (kode global masih
~1.8x, belum tuntas). **Migrasi JIT ke IR linear** SELESAI dan performanya
setara JIT produksi (setelah koreksi kesalahan benchmark awal — lihat
docs/IR.md). **AOT langsung dari IR** juga SELESAI — `isoteri bangun`
sekarang generate binary yang jalan lewat bytecode+JIT dari IR linear yang
sama, diverifikasi 17/17 + performa setara AOT lama. Benchmark suite ada di
`benchmarks/` (`jalankan.sh`), termasuk kasus regresi rekursif yang pernah
menangkap bug nyata.
Belum: variabel global/top-level masih overhead ~1.8x (stack scheduling
versi lebih canggih), dan AOT masih lex/parse/resolve saat runtime (belum
serialisasi IR statis ke binary).

**Milestone B — "Isoteri Web"** (setelah A cukup stabil) — **SELESAI**
```
DOM -> Event -> Fetch -> WebSocket -> Storage -> Canvas -> Web Worker
```
Prasyarat (`isoteri ekspor-web` + `isoteri-vm.js` + konvensi
`<script type=isoteri-web>`) sudah terbukti jalan (lihat `runtime/web/`).
**DOM + Event + Storage + Fetch (async) + Canvas 2D + WebSocket semuanya
SELESAI** — fungsi bawaan datar (`dom_pilih`, `dom_atur_teks`, `dom_ketika`,
`simpan_lokal`, `unduh_async`, `kanvas_isi_persegi`, `ws_buka`, dst.),
diimplementasikan SEPENUHNYA di `runtime/web/isoteri-vm.js` tanpa menyentuh
compiler Rust sama sekali (lihat `runtime/web/README.md` buat daftar lengkap
& `runtime/web/contoh_dom.iso`/`contoh_kanvas_ws.iso` buat contoh).
Diverifikasi lewat smoke test dengan DOM/Canvas/WebSocket tiruan (elemen,
konteks gambar, event listener, localStorage, koneksi soket async) — semua
jalur bekerja benar termasuk callback yang dipicu belakangan secara async.
Belum: Web Worker (jarang genuinely dibutuhkan buat kelas program yang
biasanya ditulis di Isoteri sejauh ini) — dan sintaks method-call
`objek.metode()` (saat ini semua lewat fungsi bawaan datar, karena parser
belum dukung pemanggilan setelah akses field — lihat catatan arsitektur di
`runtime/web/README.md`, sengaja ditunda dulu, bukan prioritas). **PENTING**:
DOM binding TIDAK masuk ke compiler/core language — masuk ke lapisan
`runtime/web/` terpisah (lihat "Arsitektur berlapis" di atas), supaya core
Isoteri tetap tidak tahu apa itu DOM.

**Milestone C — "Isoteri Ecosystem"** (baru setelah A & B) — **v1 SELESAI**
```
isoteri init / tambah / bangun / uji / format  ->  registry, LSP, VS Code, debugger, docs
```
Package manager sengaja MINIMAL dulu (`isoteri.toml` + `isoteri tambah`),
bukan langsung sekelas Cargo/npm. **Sudah ada**: `isoteri init` (scaffold
proyek: `isoteri.toml` + `src/main.iso`), `isoteri tambah nama path`
(dependensi LOKAL lewat path), resolusi `muat "nama_paket"` otomatis lewat
manifest (dicari ke atas seperti Cargo mencari Cargo.toml) di SEMUA jalur
eksekusi (native, `bangun`/AOT, `ekspor-web`) — dan `isoteri uji` (test
runner minimal: tiap `.iso` di `tes/` satu kasus uji, `gagal_uji("pesan")`
buat assertion, exit code nonzero kalau ada yang gagal, siap dipakai CI).
`isoteri.toml` di-parse pakai parser tulisan tangan (bukan dependensi crate
`toml`), konsisten dengan gaya proyek ini yang sudah menulis parser JSON
sendiri juga. Diverifikasi end-to-end: proyek dengan dependensi lokal
berhasil jalan lewat SEMUA jalur (native/AOT/web), termasuk kasus uji yang
sengaja gagal buat memastikan pelaporannya benar.

**Registry paket (v1, GIT-BASED) juga SELESAI** — keputusan arsitektur
(dibahas eksplisit sebelum implementasi): TIDAK bikin server indeks
terpusat kayak npm/crates.io (beban operasional yang belum perlu di tahap
ini), tapi model Git-based mirip Go modules/Deno — paket = repo Git APAPUN
(GitHub/GitLab/Gitea/server pribadi) dipin ke satu `tag` (rilis semver)
ATAU `rev` (commit hash), lewat `isoteri tambah nama --git URL --tag vX.Y.Z`
(atau `--rev <hash>`). `isoteri.toml` sekarang punya dua bentuk dependensi:
`{ path = "..." }` (lokal, tidak berubah) dan `{ git = "...", tag/rev =
"..." }` (registry) — direpresentasikan sebagai enum `SumberDependensi` di
compiler, satu choke point resolusi (`resolusi_muat` → `resolusi_paket_git`
kalau bukan lokal) dipakai SEMUA jalur (native/AOT/web), sama seperti
dependensi lokal. Paket diambil (`git clone`) ke cache lokal
`~/.isoteri/cache/` (override lewat env `ISOTERI_CACHE_DIR`, dipakai buat
testing/CI); `isoteri tambah --git` mengambil paketnya SAAT ITU JUGA (gagal
cepat kalau URL/tag/rev salah), bukan nunggu sampai `muat` dipanggil nanti.
CACHE = PIN: sekali tag/rev tertentu ke-cache, tidak di-fetch ulang lagi —
kalau upstream memindahkan tag ke commit lain itu di luar kendali kita
(sama seperti Go modules), didokumentasikan di KETERBATASAN.md, bukan
disembunyikan. Diverifikasi end-to-end pakai repo git lokal simulasi
"remote" berisi dua tag berbeda: `--tag` & `--rev` sama-sama berhasil
clone+checkout+jalan lewat `muat`, cache-hit terverifikasi tidak fetch
ulang di run berikutnya, dan kegagalan (tag salah/URL salah/git tidak
terinstal/kombinasi `path`+`git` atau `tag`+`rev` sekaligus di manifest)
semuanya gagal cepat dengan pesan jelas + folder cache rusak dibersihkan
otomatis (tidak menghalangi percobaan ulang).

**`isoteri format` juga SELESAI** — "formatter adalah sumber kebenaran gaya
penulisan" (bukan alat bantu opsional): cetak ulang dari AST (bukan
normalisasi teks apa adanya), jadi hasilnya SELALU konsisten terlepas dari
gaya penulisan asli. Tantangan utama: Lexer/Parser produksi membuang
komentar (`catatan: ...`) sepenuhnya — formatter naif akan diam-diam
menghapus semua komentar. Solusinya: `Lexer::tokenize_dengan_komentar()`
(method BARU terpisah, Lexer/Parser produksi TIDAK diubah sedikit pun)
mempertahankan komentar sebagai token, formatter memetakannya ke nomor
baris, membuangnya dari stream, lalu memberi sisanya ke `Parser` yang sama
persis dipakai compiler — dan menempelkan komentar kembali berdasarkan
nomor barisnya. Diverifikasi ketat: **17/17 program contoh** di proyek ini
diformat ulang (dogfooding — bukan cuma file uji terpisah) dan hasilnya
**semantically identical** (dibandingkan output eksekusi sebelum/sesudah,
bukan cuma "berhasil di-parse"), **idempoten** (format dua kali = tidak
berubah lagi), dan kasus presedensi operator (termasuk unary minus, kurung
campuran, `dan`/`atau`) tercetak ulang dengan kurung minimal tapi tetap
benar. Ditemukan & diperbaiki 1 bug nyata selama proses ini: trailing comma
setelah field terakhir `bentuk` — Isoteri (beda dari Rust) TIDAK mendukung
trailing comma, jadi formatter awal yang menambahkannya malah merusak file
yang diformat. Keterbatasan v1 (didokumentasikan, bukan disembunyikan):
komentar harus di baris sendiri (komentar sebaris dengan kode DITOLAK
dengan pesan jelas, bukan diam-diam dibuang/salah tempat); `--cek` mode
buat CI (exit code nonzero kalau ada berkas belum rapi).

Belum: index/discovery server buat registry (search "paket apa saja yang
ada" — bisa ditambah belakangan, migrasinya mulus karena registry v1 tidak
bergantung padanya), LSP, VS Code extension, debugger, documentation
generator.

### Yang sengaja DITUNDA (bukan dihapus dari visi, cuma bukan prioritas sekarang)

- **Borrow checker ala Rust** — mulai dari automatic memory management (sudah
  ada) → escape analysis compiler → baru manual/explicit ownership → borrow
  checking cuma kalau memang terbukti perlu. Isoteri **memanfaatkan**
  keunggulan Rust, bukan **mengimplementasikan ulang** Rust (lihat Hukum 8).
- **`unsafe {}` / `akses_memory()`** — ditunda sampai native backend & memory
  model benar-benar matang; API unsafe yang datang terlalu dini justru
  mengunci desain sebelum waktunya.
- **SIMD** — jangan diulang sebagai fitur ad-hoc di level AST/JIT seperti
  eksperimen sebelumnya (lebih lambat, direvert). Baru masuk akal setelah
  IR linear+typed (Milestone A) ada, supaya compiler yang MEMUTUSKAN apakah
  vectorization benar-benar untung, bukan "ada array → pakai SIMD".
