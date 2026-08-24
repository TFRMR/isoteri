# Roadmap Isoteri

Roadmap ini adalah peta eksplorasi, bukan janji jadwal. Status dan prioritas
dapat berubah berdasarkan hasil eksperimen dan kontribusi komunitas.

## Arah strategis: kenapa Isoteri DIBUTUHKAN (bukan cuma "bahasa lain")

**Ditulis setelah diskusi mendalam soal identitas & niche -- dicatat di sini
supaya arah ini tidak hilang di tengah pekerjaan teknis kecil-kecil.**

Isoteri TIDAK diposisikan sebagai pengganti JavaScript di web (itu perlombaan
yang tidak akan menang -- ekosistem JS/npm terlalu besar & matang). Isoteri
juga BUKAN "JS dengan sintaks Indonesia" -- itu cuma gaya, bukan alasan
dibutuhkan. Nilai intinya harus lebih tajam dari itu.

### Masalah nyata yang mau dipotong

Di stack web modern (apapun bahasa backend-nya -- Node, Python, Go), ada
pemborosan struktural yang SELALU terjadi: skema data & aturan validasi
ditulis **dua kali** -- sekali di frontend (TypeScript + Zod/Yup, buat UX,
validasi instan sebelum submit) dan sekali lagi di backend (bahasa lain,
wajib, karena frontend tidak bisa dipercaya). Dua penulisan itu gampang
kehilangan sinkron -- sumber bug klasik ("kenapa validasi form beda dari
validasi API?"). Ini bukan soal gaya penulisan, ini SATU KELAS BUG yang
melekat di arsitektur client-server manapun, di bahasa manapun.

### Kenapa Isoteri kebetulan punya properti arsitektur yang pas buat ini

Isoteri sudah (per hari ini) punya SATU compiler yang bisa menghasilkan
target native/AOT (buat backend, tervalidasi lewat `isoteri bangun`) DAN
target browser (bytecode/WASM, tervalidasi lewat ekspor-web +
`isoteri-vm.js`) dari SATU SUMBER yang sama, tanpa rewrite. Artinya:
definisi `bentuk` + fungsi validasi bisa ditulis SEKALI, di-`muat` dari
kedua sisi (frontend & backend), dan DIJAMIN selalu sinkron karena memang
cuma ada satu salinan kodenya -- bukan disiplin tim yang harus dijaga
manual.

### Klaim "lebih cepat" yang jujur (bukan overclaim)

Isoteri BELUM bisa diklaim lebih cepat dari JS murni **di dalam browser**
(saat ini `isoteri-wasm` cuma memindahkan COMPILER ke browser -- hasilnya
tetap dieksekusi interpreter JS `isoteri-vm.js`, bukan instruksi WASM asli
-- lihat item "Backend WASM asli" di bagian IR). Klaim cepat yang JUJUR dan
bisa dibuktikan itu ada di **sisi backend/server**: logika yang sama bisa
di-AOT-compile jadi binary native atau di-JIT (Cranelift) -- itu beneran
lebih cepat dari backend Node.js/Python buat logika berat, karena tidak
lewat interpreter bahasa dinamis sama sekali. Belum ada benchmark head-to-head
yang mempublikasikan angka ini -- lihat item di bawah.

### Melengkapi, bukan menyaingi

JS tetap pegang peran yang dia jago: interaktivitas UI, ekosistem
library visual/animasi/DOM kompleks. Isoteri numpang di situ lewat rencana
interop JS (lihat item di bawah). Yang Isoteri ambil alih cuma LAPISAN
DEFINISI TIPE + ATURAN BISNIS + BACKEND -- potongan yang di stack JS
biasanya dipecah jadi 3 hal terpisah (TypeScript types, Zod/Yup validation,
Node/Python backend) yang harus dijaga manual tetap sinkron. Isoteri
menyatukannya jadi satu penulisan.

### Prasyarat yang masih harus dibangun supaya cerita ini nyata (urutan prioritas)

1. **Interop JS/npm -- SELESAI & TERVALIDASI.** `js_global`, `js_panggil`,
   `js_panggil_bebas`, `js_baru`, `js_ambil`, `js_atur`, `js_ke_peta`
   (`runtime/web/isoteri-vm.js`, murni JS seperti fitur DOM lainnya --
   tidak ada representasinya di Rust, browser-only). Scope: library yang
   nempel ke `window` lewat CDN (bukan sistem import/bundler modern).
   Konversi nilai dua arah otomatis (primitif, Daftar<->array, Peta<->
   object JS literal, ElemenDOM<->Element JS asli, closure Isoteri<->
   callback JS asli dengan batasan 1 argumen pertama saja diteruskan).
   Objek/fungsi JS "hidup" dibungkus jadi handle (`Instans "JsObjek"`,
   pola sama dengan `domRegistry`/`listenerRegistry`), bukan langsung
   dikonversi jadi Peta -- supaya kemampuan panggil method/baca properti
   live-nya tidak hilang. Diverifikasi lewat `runtime/web/uji_interop_js.html`
   (5 kasus, semua LULUS): `Math.max` multi-argumen, constructor `Date` +
   panggil method pada instansnya, baca/tulis properti objek, konversi
   otomatis Peta Isoteri -> object JS (viaJSON.stringify), dan closure
   Isoteri dipanggil sebagai callback asli oleh `setTimeout` milik JS.
   Lihat `runtime/web/README.md` bagian "Interop JS" untuk dokumentasi &
   referensi fungsi lengkap.
2. **HTTP server dasar -- SELESAI & TERVALIDASI.** `server_mulai(port,
   handler)` dan `respons_status(kode, nilai)` (`src/lib.rs`, dispatch di
   `Instr::PanggilBawaan` -- punya akses `pustaka`/`state` buat manggil
   handler berulang kali per request, pola yang sama dipakai
   `petakan`/`saring`/`urutkan`). Pakai crate `tiny_http` (fitur Cargo baru
   "native-server", default ON untuk native, otomatis OFF untuk wasm32 --
   sudah diverifikasi compile bersih tanpa fitur ini juga, jadi
   `isoteri-wasm/` tidak kebawa dependency yang tidak relevan). BLOCKING
   secara sengaja (konsisten dengan model eksekusi VM yang sinkron, sama
   seperti `unduh()`) -- TIDAK ada runtime async/tokio.

   `handler` menerima SATU argumen: Peta berisi `"metode"`, `"path"`,
   `"query"` (Peta), `"header"` (Peta), `"body"` (Teks). Nilai balik
   diinterpretasi otomatis: `Teks` -> 200 text/plain, `Peta`/`Daftar`/
   `Instans`/dst -> 200 application/json (di-serialize lewat mesin JSON
   yang SAMA dipakai `tulis_berkas()`, bukan encoder baru), `Kosong` -> 204,
   dibungkus `respons_status(kode, nilai)` -> status custom. Error di dalam
   handler TIDAK mematikan server -- request itu dibalas 500, server tetap
   jalan buat request berikutnya.

   Diverifikasi: (1) 17/17 program contoh masih cocok golden output (nol
   regresi), (2) build tanpa fitur `native-server` tetap compile bersih
   (simulasi kondisi wasm32), (3) diuji fungsional sungguhan lewat `curl`
   ke server yang benar-benar jalan: GET Teks -> text/plain benar, GET Peta
   -> auto JSON dengan Content-Type benar, `respons_status(404, ...)` ->
   HTTP 404 benar, POST ke path custom -> metode & path di `req` terbaca
   benar. Lihat `docs/REFERENSI.md` bagian "HTTP Server" untuk dokumentasi
   & tabel referensi lengkap.
3. **Contoh nyata + dokumentasi pola "satu skema, dua sisi"** -- prasyarat
   (interop JS + HTTP server) sekarang SUDAH ADA. Buktikan lewat contoh
   konkret (bukan cuma teori): `bentuk` + fungsi validasi yang sama dipakai
   identik di form browser dan endpoint backend. **Prioritas berikutnya.**
4. **Benchmark backend Isoteri (AOT) vs Node.js/Python** untuk beban kerja
   yang representatif -- supaya klaim "lebih cepat" punya angka publik,
   bukan janji.
5. **Backend WASM asli** (compile IR Isoteri langsung ke instruksi WASM,
   bukan bytecode JSON yang ditafsirkan `isoteri-vm.js`) -- baru ini yang
   bisa membuka klaim "lebih cepat" DI DALAM browser juga, bukan cuma di
   backend. Prioritas lebih rendah dari 4 poin di atas karena backend dulu
   yang punya cerita jelas.

Item-item lain di roadmap ini (namespace modul lengkap, LSP/tooling editor,
semver registry v2, dst.) tetap berguna dan tetap dikerjakan, tapi arah di
atas ini yang jadi KOMPAS -- kalau ada pilihan mengerjakan item mana
duluan dan tidak jelas, dahulukan yang paling dekat mendukung 5 poin di
atas.

### Fondasi jangka panjang: identity, effect, provenance (ditanam dini, bukan ditempel belakangan)

**Dicatat setelah diskusi mendalam soal menanam fondasi analisis statis di
IR/runtime SEKARANG (selagi arsitektur masih muda & bebas diubah), bukan
sebagai fitur security yang ditempel belakangan.** Tiga hal, urutan
prioritas berdasar rasio manfaat:biaya:

1. **Stable Identity** -- setiap operasi bisa ditelusuri balik ke source
   code. Sudah SETENGAH JALAN: nomor baris per statement sudah ada di
   seluruh pipeline (`(usize, Stmt)`, dipakai semua pesan error "Baris
   N: ..."). Yang belum: identitas stabil sampai level
   instruksi/ekspresi yang BERTAHAN lewat `optimisasi_blok` (mirip debug
   info gaya DWARF). Manfaat bukan cuma buat analisis keamanan nanti --
   ini juga jalan pintas ke source map buat debug WASM di browser (celah
   yang dicatat di bagian WASM) dan pesan error yang lebih presisi.
   **Worth dikerjakan, biaya moderat.**
2. **Effect & Boundary** -- compiler tahu operasi mana yang `pure`,
   `network`, `filesystem`, `database`, dst. INI TITIK KUAT ISOTERI SECARA
   KEBETULAN: builtin adalah himpunan TERTUTUP (satu `match nama { ... }`
   di `Instr::PanggilBawaan`/`panggil_bawaan`, tidak ada reflection atau
   FFI sembarang) -- beda jauh dari JS/Python yang effect-tracking-nya
   susah sound karena ekosistem paket nyaris tak terbatas. Bikin tabel
   statis `nama_builtin -> kategori_efek` itu kerja DEFINISI, bukan
   riset. Propagasi lewat fungsi buatan user (whole-program, fixed-point
   atas call graph) lebih berat, tapi PAS nempel di titik yang sudah ada:
   `ekspansi_muat` sudah meratakan semua modul jadi satu program flat
   sebelum compile -- itu titik alami buat pass analisis whole-program.
   **Sinergi langsung dengan arah strategis "satu skema, dua sisi" di
   atas**: compiler yang bisa JAMIN fungsi validasi client-side benar-benar
   `pure` (nggak nyelip panggil jaringan/filesystem diam-diam) memperkuat
   cerita itu, bukan proyek yang bersaing arah. **Worth dikerjakan,
   sinergi tinggi dengan kompas utama.**
3. **Provenance/taint tracking** -- compiler tahu asal aliran data (input
   HTTP, file, database, parameter). INI YANG DISCOPE ULANG, bukan
   dikerjakan penuh: taint tracking granular PER-VALUE (nempel tag di tiap
   `Value`) punya dua masalah nyata -- (a) biaya memori/performa, langsung
   berlawanan arah dengan kerjaan representasi flat (`DaftarAngka`/
   `DaftarDesimal`, 32 byte -> 8 byte per elemen) yang baru selesai; (b)
   taint analysis yang SOUND itu masalah riset -- bahkan tool besar
   (CodeQL, Semgrep) masih sering false-positive/negative karena implicit
   flow, sanitization detection, alias analysis. **Rekomendasi: turunkan
   granularitas dari hasil poin #2** -- lacak di level FUNGSI (fungsi mana
   yang bersentuhan dengan `req` parameter HTTP, `baca_berkas`, `unduh`,
   dst), bukan di level value individual. Jauh lebih murah, masih berguna
   buat kasus pakai yang dituju (`isoteri test --security`, bug tracing),
   TANPA butuh riset taint-analysis penuh. **Revisit sebagai proyek
   terpisah setelah #1 dan #2 solid -- jangan mulai dari sini.**

Potensi jangka panjang (belum dikerjakan, dicatat biar tidak hilang):
`isoteri test --security`, bug tracing berbasis effect graph, capability
security (fungsi cuma boleh dipanggil kalau efeknya diizinkan pemanggil),
AI bug finder yang jalan di atas graph effect+identity ini tanpa perlu
membongkar arsitektur VM/compiler yang sudah ada.

---

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
- [x] **`dom_ketika()` sekarang bisa `removeEventListener` -- SELESAI & TERVALIDASI.**
  `dom_ketika(el, event, callback)` sekarang mengembalikan sebuah handle
  (`Instans "PawangEvent"`, bukan `Kosong` seperti sebelumnya) yang bisa
  disimpan lalu dilewatkan ke fungsi baru `dom_hapus_ketika(handle)` buat
  melepas listener tersebut. Implementasinya di `runtime/web/isoteri-vm.js`
  (murni JS -- fitur DOM ini memang tidak ada representasinya di Rust sama
  sekali, browser-only): referensi fungsi JS asli yang dibungkus
  `addEventListener()` disimpan di `listenerRegistry` (pola yang sama
  dengan `domRegistry` yang sudah ada buat elemen DOM), supaya
  `removeEventListener()` bisa dipanggil dengan referensi fungsi yang
  PERSIS SAMA (syarat wajib JS -- kalau bukan referensi yang sama,
  `removeEventListener` diam-diam gagal tanpa error). Melepas handle yang
  sudah dilepas (atau dipanggil dua kali) tidak error -- idempotent, sama
  seperti `dom_hapus()` terhadap elemen yang sudah hilang. Sistem listener
  komponen (`_komponenPasangDelegasi`, dipakai `komponen_pasang`/
  `komponen_lepas`) TIDAK terpengaruh -- itu jalur terpisah yang sudah
  pakai pola cleanup sendiri sejak awal. Diverifikasi lewat
  `runtime/web/uji_hapus_ketika.html`: listener terbukti berhenti merespons
  event (termasuk event yang dipicu programatik) tepat setelah
  `dom_hapus_ketika()` dipanggil.
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
- [x] **Evaluasi namespace modul -- SELESAI & TERVALIDASI.** `muat "path" sebagai alias`
  (baru): fungsi top-level modul diakses lewat `alias.fungsi(...)`, tidak numplek ke namespace
  global -- dua modul independen boleh punya fungsi bernama sama tanpa bentrok. `muat "path"`
  tanpa alias (lama) tetap 100% seperti sebelumnya (flat, backward-compatible penuh). Scope
  sengaja dibatasi ke FUNGSI saja untuk sekarang (belum `bentuk`/variabel global lewat alias) --
  lihat docs/REFERENSI.md untuk detail & batasan.

  **Temuan arsitektur penting sewaktu implementasi**: ternyata ada DUA implementasi `muat` yang
  independen dan berpotensi berbeda kelakuan -- `ekspansi_muat` (berbasis AST, dipakai
  `jalankan_berkas`) dan `kumpulkan_sumber_gabungan`/`kumpulkan_rekursif` (berbasis PENCOCOKAN
  TEKS MENTAH per baris, dipakai `jalankan_berkas_via_ir`, `ekspor_json_dari_berkas`, DAN
  `isoteri bangun` di main.rs). Implementasi teks itu buta total soal `sebagai alias` (langsung
  dibuang tanpa diproses) -- kalau dibiarkan terpisah, fitur alias ini TIDAK akan bekerja di 3
  dari 4 jalur eksekusi (IR/JIT, ekspor-web, AOT build), termasuk jalur web/WASM yang baru saja
  divalidasi minggu ini. Diperbaiki dengan menyatukan SEMUA jalur lewat satu fungsi
  `program_dari_berkas()` (AST-based) -- `kumpulkan_sumber_gabungan`/`kumpulkan_rekursif` DIHAPUS
  sepenuhnya (bukan cuma tidak dipakai) supaya tidak ada risiko dua implementasi paralel diam-diam
  berbeda lagi di masa depan.

  Rincian teknis: fungsi top-level modul beralias di-mangle secara internal dengan pola
  `__modul_<alias>__<nama>` (BUKAN titik seperti percobaan pertama -- titik gagal roundtrip
  lewat teks karena lexer selalu memecahnya jadi token terpisah, ketahuan lewat kegagalan nyata
  di jalur `isoteri bangun` yang butuh cetak-ulang AST jadi teks source lalu di-parse ulang dari
  nol saat binary hasil build dijalankan). `Expr::PanggilMetode` (AST baru buat `x.y(args)`)
  general-purpose: kalau `x` alias modul dikenal -> panggilan fungsi langsung (mangled); kalau
  bukan -> "panggil NILAI di field itu" (mis. closure disimpan di field bentuk), lewat mekanisme
  `PanggilNilai` yang sudah ada -- manfaat sampingan, bukan cuma buat modul.

  Diverifikasi: (1) 17/17 program contoh masih cocok persis dengan golden output (nol regresi),
  (2) dua modul beda nama fungsi sama diakses lewat alias masing-masing, output benar, (3)
  fungsi di dalam modul beralias saling panggil satu sama lain dengan benar (mangling konsisten
  di definisi maupun call site internal), (4) alias dipakai dua kali -> error jelas, (5) `muat`
  tanpa alias tetap identik perilaku lama, (6) hasil identik di SEMUA EMPAT jalur eksekusi (CLI
  bytecode biasa, `via-ir`, `ekspor-web` + dijalankan via `isoteri-vm.js` lewat Node, DAN
  `isoteri bangun` -- AOT native binary dijalankan langsung).
- [x] **Representasi data numerik yang lebih flat -- SELESAI & TERVALIDASI.**
  `Value` punya 2 varian baru: `DaftarAngka(Rc<Vec<i64>>)` dan
  `DaftarDesimal(Rc<Vec<f64>>)`. Literal daftar (`[1, 2, 3]`) otomatis naik
  level ke representasi ini kalau semua elemennya homogen Angka/Desimal
  (lihat `coba_promosikan_flat()` / `buat_daftar()`), turun balik ke
  `Value::Daftar` biasa kalau tipenya campuran. Semua operasi List yang
  sudah ada (indexing baca/tulis, `petakan`/`saring`/`urutkan`, `gabung`,
  `ambil`, `panjang`, `ulang setiap`, `ulang selaras`, perbandingan `==`,
  ekspor JSON ke `isoteri-vm.js`) tetap benar lewat fallback materialisasi
  (`daftar_materialisasi()`) untuk operasi yang belum punya jalur cepat
  native. Diverifikasi: (1) regresi nol -- 17/17 program contoh cocok
  persis dengan golden output sebelum perubahan, (2) uji fungsional
  tambahan (indexing, mutasi, `petakan`+`saring` berantai, `ulang setiap`,
  `==`, `gabung`) semua benar, (3) ekspor JSON ke `isoteri-vm.js`
  byte-identik (degradasi otomatis balik ke format `{"t":"Daftar",...}`
  biasa, JS tidak perlu tahu soal representasi flat internal ini sama
  sekali).
- [x] **Bug sampingan ditemukan & diperbaiki: `isoteri ekspor-web` nondeterministic.**
  Ditemukan selama verifikasi representasi flat di atas (bukan disebabkan
  olehnya -- sudah ada sebelumnya, terverifikasi dengan mengetes binary versi
  LAMA dua kali dan hasilnya beda juga). Akar masalah: urutan fungsi dalam
  bundel `.isoweb.json` diambil dari `resolver.fungsi_out.keys().cloned()`
  yaitu iterasi `HashMap<String, ...>` -- Rust sengaja mengacak urutan
  iterasi HashMap antar-run (proteksi DoS), jadi index fungsi bisa
  berbeda-beda tiap kali source yang SAMA di-compile ulang. Bukan bug yang
  bikin program salah jalan (eksekusinya tetap benar via `isoteri-vm.js`,
  diverifikasi lewat `node runtime/web/jalankan-node.js`), tapi bikin
  `.isoweb.json` tidak reproducible byte-level -- `git diff` selalu nunjukin
  perubahan walau logikanya identik. Perbaikan: tambah `nama_fungsi.sort()`
  di 4 titik compile entry (bytecode biasa, `jalankan_stmt_list`, jalur IR,
  ekspor-web) sebelum index fungsi ditetapkan. Diverifikasi: compile source
  yang sama 5x berturut-turut sekarang menghasilkan `.isoweb.json`
  byte-identik semua (sebelumnya selalu beda).
- [ ] Semver range di package registry (v2) -- v1 git-based/pin-exact-tag sudah selesai

## Eksperimen performa

- [ ] Benchmark VM vs JIT pada workload nyata
- [ ] Benchmark Isoteri vs implementasi pembanding yang relevan
- [x] **Eksperimen representasi `Daftar` numerik -- SELESAI, lihat di atas.**
  Diukur dua cara: (1) microbenchmark Rust terisolasi (cuma loop sum
  murni) -- speedup **9-10x** untuk `Vec<i64>` flat vs `Vec<Value>` tagged
  (compiler auto-vectorize loop integer flat, TIDAK bisa untuk yang
  tagged); (2) end-to-end lewat VM sungguhan (`jumlah()` dipanggil
  berulang di daftar 20.000 elemen) -- speedup jauh lebih kecil, **cuma
  ~1.1-1.15x**, karena overhead dispatch per-panggilan-fungsi interpreter
  (pencocokan nama fungsi, penyusunan argumen, instruksi sekitar seperti
  `BinOp`/`StoreGlobal`) mendominasi total waktu untuk ukuran daftar
  segini -- bukan loop sum-nya sendiri. **Kesimpulan jujur**: manfaat
  MEMORI (4x lebih hemat, `sizeof(Value)`=32 byte vs `sizeof(i64)`=8 byte)
  berlaku tanpa syarat; manfaat KECEPATAN baru terasa besar kalau daftarnya
  sangat besar dan/atau operasi numerik jadi bottleneck nyata -- bukan
  lompatan performa dramatis untuk pola pemakaian tipikal (banyak
  panggilan fungsi kecil-kecil).
- [x] Evaluasi SIMD hanya jika representasi data mendukungnya -- prasyarat
  (representasi flat) sekarang SUDAH ADA. Percobaan SIMD eksplisit
  sebelumnya (AVX2 manual) gagal karena representasi lama; belum dicoba
  ulang di atas representasi flat yang baru ini -- auto-vectorization
  compiler standar (tanpa SIMD intrinsic manual) sudah memberi sebagian
  besar manfaat untuk `DaftarAngka` (lihat speedup 9-10x di atas). SIMD
  manual eksplisit kemungkinan baru berguna kalau bottleneck-nya memang di
  loop sum besar, bukan overhead dispatch VM -- lihat catatan di atas.

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

Belum: `pkg/` hasil build juga belum di-commit permanen ke lokasi final di
repo/CI (saat ini disalin manual ke `runtime/web/pkg/` di mesin lokal).

**Update: validasi diperluas ke fitur kompleks -- SELESAI.** Lewat
`runtime/web/uji_wasm_lanjutan.html`, tiga kasus uji dijalankan lewat WASM
di browser dan dibandingkan otomatis terhadap output referensi dari CLI
native (`program_bentuk.iso`, `program_closure.iso`,
`program_list_lanjutan.iso`): (1) struct/`bentuk` + `coba/tangkap` untuk
field yang tidak ada, (2) closure berlapis termasuk closure disimpan di
Daftar dan closure nested 2 level, (3) fungsi higher-order
(`petakan`/`saring`/`urutkan`) termasuk `urutkan` pakai kunci custom pada
struct. Ketiganya COCOK persis dengan referensi native. Yang sengaja TIDAK
diuji lewat WASM: `tulis_berkas`/`baca_berkas` (`program_lanjutan.iso`) --
ini bukan bug, browser memang tidak punya filesystem, jadi fitur ini by
design cuma jalan di CLI native. DOM binding penuh (`peristiwa`,
`dom_ketika`, dll) juga belum diuji lewat jalur WASM secara spesifik.

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

