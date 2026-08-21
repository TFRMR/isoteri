# Keterbatasan yang Diketahui

Dokumen ini mengumpulkan semua batasan bahasa Isoteri yang sudah teridentifikasi sampai saat ini — supaya tidak perlu ditemukan ulang dari nol, dan supaya pengguna tahu apa yang **memang belum didukung** vs **kemungkinan bug baru**. Semua yang tercantum di sini sudah diverifikasi lewat pengujian langsung, bukan dugaan.

Kalau kamu menemukan perilaku aneh yang **tidak** ada di daftar ini, kemungkinan itu bug baru, bukan batasan yang diketahui.

---

## Bahasa & Semantik

### Overflow `Angka` -- sekarang error runtime jelas & konsisten di SEMUA jalur eksekusi (bytecode VM, JIT produksi, via-ir, AOT)
```
ingat besar = 9223372036854775807   catatan: i64::MAX
tampilkan besar + 1                  catatan: error jelas "Angka meluap (overflow)"
```
`+`, `-`, `*` buat `Angka` pakai `checked_add`/`checked_sub`/`checked_mul` di eksekusi normal (bytecode VM) DAN di `ulang selaras`, termasuk kalau overflow-nya kejadian saat compile-time constant-folding (mis. `tampilkan 9223372036854775807 + 1` langsung ketahuan sebelum program jalan). Bonus temuan dari sesi checked-arithmetic: literal angka yang gagal di-parse (kegedean buat `i64`, mis. salah ketik nambahin banyak digit) DULU diam-diam jadi `0` di lexer -- sekarang juga error jelas ("Literal angka ... tidak valid atau di luar jangkauan").

**Sudah diperbaiki (sesi overflow-trapping JIT):** dulu fungsi yang lolos syarat JIT (parameter beranotasi tipe eksplisit, mis. `fungsi f(a: Angka, b: Angka) {...}`) di-compile Cranelift pakai instruksi `iadd`/`isub`/`imul` biasa yang TIDAK trap saat overflow, jadi wrap-around diam-diam -- beda perilaku dari fungsi identik tanpa anotasi tipe (lari ke bytecode, error jelas). Sekarang KEDUA jalur JIT (`kompilasi()`/`KompilerBadan` yang dipakai `isoteri jalankan`, DAN `kompilasi_dari_ir()`/`KompilerBadanIr` yang dipakai `isoteri via-ir` & `isoteri bangun`/AOT) pakai `sadd_overflow`/`ssub_overflow`/`smul_overflow` (BUKAN hardware trap Cranelift, karena itu SIGILL/crash seluruh proses tanpa peduli `coba/tangkap` pembungkus -- lihat catatan di `JitEngine::kompilasi`), diakumulasi ke flag per-fungsi, dan dicek pemanggil Rust (VM) saat fungsi kembali -- termasuk kalau overflow-nya kejadian di kedalaman rekursi (sudah diverifikasi: fungsi rekursif 70 level dalam yang overflow di level ke-1 tetap terdeteksi & pesannya sampai ke pemanggil paling luar), dan tetap catchable lewat `coba/tangkap` seperti jalur bytecode biasa. `Desimal` (float) TIDAK disentuh perubahan ini -- overflow float ke `+-inf` bukan silent-wraparound yang menyesatkan seperti `i64`, jadi tidak butuh trapping serupa.

Catatan tambahan: menulis literal `i64::MIN` (`-9223372036854775808`) langsung juga masih kena keterbatasan umum bahasa lain -- `-N` didesugar jadi `0 - N`, dan `N` (`9223372036854775808`) sendiri sudah kelebihan 1 dari `i64::MAX` sebagai literal POSITIF, jadi tetap error kalau ditulis langsung. Solusinya: `ingat x = -9223372036854775807 - 1`.

Catatan lain: `isoteri-vm.js` (runtime browser) merepresentasikan `Angka` sebagai `Number` JS biasa (double 64-bit), BUKAN `i64` asli seperti versi native -- jadi perilaku ekstrimnya beda lagi dari native: bukan wrap-around/error, tapi kehilangan presisi diam-diam begitu lewat `Number.MAX_SAFE_INTEGER` (2^53). Ini masih belum diperbaiki -- perlu `BigInt` buat benar-benar menyamai semantik `i64`, di luar cakupan sesi overflow-trapping JIT ini (itu soal representasi angka di browser, bukan soal deteksi overflow di native).

### Negasi boolean (`!ekspr`) -- didukung (ditemukan & diperbaiki di sesi ini)
```
tampilkan !benar        catatan: salah
tampilkan !0              catatan: benar -- pakai truthiness YANG SAMA seperti 'kalau'/'dan'/'atau'
tampilkan !x.selesai      catatan: jalan buat field bentuk juga
```
Sebelumnya bahasa ini sama sekali gak punya cara negasi boolean langsung (bukan cuma belum ada `!`, tapi juga gak ada kata kunci pengganti kayak `tidak`/`bukan`) -- ketauan pas nulis contoh Component System (toggle status di Todo List). `!` sekarang jadi operator unary sungguhan (presedensi tertinggi, sama seperti minus unary), dievaluasi via `Value::truthy()` yang SAMA dipakai kondisi `kalau`, jadi konsisten: `!5` salah (5 truthy), `!0`/`!""`/`![]` benar. Didukung penuh di semua jalur (native, `via-ir`/AOT, web export) -- TIDAK di-JIT (fungsi yang makai `!` fallback ke bytecode VM, karena operasi ini fundamental beda tipe balikan (Bool) dari JIT sempit yang cuma buat Angka/Desimal).

### Operator modulo (`%`), increment/decrement (`++`/`--`), compound assignment (`+=` dst.) -- didukung
```
tampilkan 17 % 5        catatan: 2
x += 1                    catatan: sama seperti x = x + 1
x++                       catatan: sama seperti x = x + 1 (HANYA statement baris sendiri, bukan ekspresi)
rek.saldo -= 30           catatan: compound assignment field juga didukung, termasuk nested
```
Semuanya gula sintaksis murni di parser (didesugar ke bentuk `nama = nama <op> nilai`), kecuali `%` yang jadi `BinOp` baru sungguhan (butuh entry di semua jalur: eval, formatter, tipe inferensi, IR, JSON export web). Modulo dengan pembagi 0 melempar error runtime jelas, sama seperti pembagian. `%` TIDAK di-JIT (sama seperti `/`, butuh cek pembagi-nol saat runtime) -- fungsi yang memakainya otomatis fallback ke bytecode VM biasa, tetap benar cuma lebih lambat dari fungsi murni aritmatika lain.

Efek samping yang perlu diketahui: karena `+=`/`++`/`--` didesugar TOTAL tanpa jejak di AST, `isoteri format` akan menormalisasi balik ke bentuk eksplisit (`total += 5` -> `total = total + 5`) -- ini bukan bug, cuma gula sintaksisnya memang tidak "diingat" formatter. `++`/`--` cuma didukung buat variabel (`i++`), belum buat field (`objek.field++`).

### `putus`/`lanjut` (break/continue) -- didukung di SEMUA jalur eksekusi (native, `via-ir`, AOT, web export)
Sudah bisa dipakai di `ulang` dan `ulang setiap` (loop terdekat, boleh bersarang, aman dipakai di dalam `coba/tangkap` -- handler_stack VM ditutup lewat `Instr::TutupHandler` yang disisipkan sebelum lompat kalau melompat keluar dari tengah `coba` aktif):
```
ulang (i < 10) {
    i = i + 1
    kalau (i == 3) { lanjut }   catatan: lompat ke iterasi berikutnya
    kalau (i == 7) { putus }    catatan: keluar loop
    tampilkan i
}
```
**Sudah diperbaiki (sesi putus/lanjut di via-ir/AOT):** dulu `isoteri via-ir` dan `isoteri bangun` (AOT, yang secara internal lewat jalur `via-ir` juga -- lihat `jalankan_sumber_via_ir` di kode yang di-generate) PANIK (crash, bukan graceful error) kalau ketemu `putus`/`lanjut` -- jalur IR-linear (`IrLower`) belum diimplementasikan buat dua statement ini. Sekarang `IrLower` punya `loop_stack`/`LoopCtxIr` (menyimpan target lompat `lanjut` & daftar backpatch `putus`, sama persis polanya dengan `Compiler::LoopCtx` di jalur bytecode biasa) dan `coba_depth` counter sendiri (buat tau berapa `Instr::TutupHandler` perlu disisipkan kalau `putus`/`lanjut` melompat keluar dari tengah `coba` aktif) -- **sudah diverifikasi**: loop bersarang, `putus`/`lanjut` di dalam `coba/tangkap` di dalam loop (kasus paling rawan -- lihat `contoh_ergonomi/putus_lanjut_di_dalam_coba.iso`), lewat `isoteri via-ir` MAUPUN binary hasil `isoteri bangun`, hasilnya identik dengan jalur `isoteri jalankan` biasa. `ulang selaras` (loop paralel) MASIH belum mendukung `putus`/`lanjut` -- evaluatornya memang sudah dibatasi terpisah, di luar cakupan perbaikan ini (beda mekanisme sama sekali, bukan soal IR lowering).

### `lainnya kalau` (else-if) -- didukung
```
kalau (a) { ... } lainnya kalau (b) { ... } lainnya { ... }
```
Ini gula sintaksis murni di parser (desugar jadi `Kalau` bersarang di dalam `lainnya`), jalan di semua jalur eksekusi termasuk `via-ir`/AOT/web export.

### Assignment lewat indeks -- didukung
```
daftar[0] = 99                catatan: bisa
peta["x"] = 99                  catatan: bisa, kunci baru otomatis ditambahkan (insert-or-update)
matriks[0][1] = 100            catatan: nested/berapa level pun boleh
objek.daftar[0] = "citra"     catatan: campur field + indeks juga boleh
daftar[0] += 5                  catatan: compound assignment lewat indeks juga jalan
```
Immutable/clone-on-write, konsisten dengan `bentuk` (`objek.field = nilai`) yang memang sudah didukung sejak awal -- assignment indeks membangun `Daftar`/`Peta` BARU di baliknya, bukan mutasi in-place. `Peta`: kunci yang belum ada otomatis di-insert. `Daftar`: indeks harus sudah ada (di luar jangkauan -> error runtime jelas, TIDAK auto-extend -- pakai `tambah()` buat menambah elemen). Jalan di semua jalur eksekusi termasuk `via-ir`/AOT/web export (numpang di mekanisme "escape hatch" yang sama dengan assignment field, karena instruksinya straight-line tanpa lompatan internal).

### Variabel global harus dideklarasikan sebelum dipakai (tekstual)
Tidak ada forward-reference untuk `ingat` di level atas — beda dari `fungsi` dan `bentuk` yang boleh dipakai sebelum baris deklarasinya (karena keduanya di-pre-scan sebelum resolusi jalan).

### Deklarasi ulang `ingat` dengan nama sama (di file/fungsi yang sama) -- sekarang error jelas
```
ingat x = 5
ingat x = 10     catatan: SEKARANG error kompilasi jelas, bukan diam-diam ketiban
x = 10             catatan: begini caranya UBAH nilai x yang sudah ada (tanpa 'ingat')
```
Berlaku juga buat parameter fungsi -- `fungsi f(x) { ingat x = 99 ... }` sekarang error juga (nama parameter dianggap sudah "dideklarasikan"). Ini konsisten dengan duplikasi nama `fungsi`/parameter/field `bentuk` yang sudah lebih dulu di-cek. Diuji terhadap seluruh 27 program contoh yang ada -- nol regresi, jadi perubahan ini aman.

---

## Closure

### Closure nested (di dalam fungsi lain) tidak bisa rekursi ke dirinya sendiri
```
fungsi buat() {
    kembalikan fungsi(n) {
        kalau (n <= 0) { kembalikan 0 }
        kembalikan diri_sendiri(n - 1)   catatan: ERROR -- nama closure-nya sendiri gak ada
    }
}
```
Closure **level atas** yang ditugaskan lewat `ingat nama = fungsi(...) {...}` **bisa** rekursi ke dirinya sendiri (karena slot globalnya didaftarkan lebih dulu sebelum badan closure diresolve). Closure yang dibuat **di dalam fungsi/closure lain** tidak bisa, karena pada saat closure-nya dibuat (snapshot capture diambil), nilai dirinya sendiri belum ada untuk ditangkap. Workaround: pakai `fungsi nama(...) {...}` biasa untuk kasus rekursif yang butuh nested.

### Capture closure itu snapshot NILAI, bukan referensi hidup
```
fungsi buat_penambah(n) {
    kembalikan fungsi(x) { kembalikan x + n }
}
```
Kalau `n` di scope pembungkus berubah SETELAH closure-nya dibuat, closure-nya tetap pakai nilai `n` pada saat ia dibuat, bukan nilai `n` yang terbaru. Ini beda dari closure di JavaScript/Python (yang capture-by-reference). Perilaku ini sengaja (konsisten dengan gaya immutable/clone-on-write di seluruh bahasa), bukan bug.

### Closure dengan capture tidak pernah dikompilasi JIT
Closure yang menangkap variabel apa pun dari scope pembungkusnya otomatis jalan lewat bytecode VM, meski semua tipe datanya numerik. Cuma closure **tanpa capture sama sekali** (biasanya closure level atas) yang berpeluang dikompilasi JIT, dengan syarat sama seperti fungsi biasa (lihat [REFERENSI.md](REFERENSI.md#kompilasi-jit)).

### `petakan()`/`saring()`/`urutkan()` -- sekarang menerima closure langsung
```
petakan(daftar, fungsi(n) { kembalikan n * n })   catatan: bisa, closure inline
ingat genap = fungsi(n) { kembalikan n % 2 == 0 }
saring(daftar, genap)                               catatan: bisa, closure lewat variabel
petakan(daftar, "kuadrat")                          catatan: cara lama tetap bisa, nama fungsi via Teks
ingat ambang = 3
saring(daftar, fungsi(n) { kembalikan n > ambang }) catatan: bisa, closure DENGAN capture juga jalan
```
Argumen kedua ketiga fungsi ini sekarang menerima Teks (nama fungsi, cara lama) ATAU closure first-class (`Value::Fungsi`) sekaligus -- kalau closure-nya punya tangkapan (capture), itu otomatis disambung transparan di belakang layar, jadi yang perlu dipikirkan pengguna cuma argumen terakhir (item daftarnya). Jalan di semua jalur eksekusi termasuk `via-ir`/AOT/web export (numpang di `PanggilBawaan`, sudah lewat escape hatch `Legacy`).

Yang **masih belum** bisa: melewatkan nama fungsi top-level TANPA tanda kutip sebagai nilai (mis. `petakan(daftar, kuadrat)` tanpa closure literal atau string) -- `kuadrat` di situ akan dicari sebagai variabel dan gagal, karena fungsi top-level bukan first-class value secara otomatis. Kalau perlu, bungkus jadi closure kecil: `petakan(daftar, fungsi(x) { kembalikan kuadrat(x) })`, atau tetap pakai bentuk Teks lama `petakan(daftar, "kuadrat")`.

---

## Modul (`muat`)

### Satu ruang nama global, tanpa prefix per modul
Tidak ada `matematika.kuadrat()` — begitu `muat "matematika.iso"`, fungsi `kuadrat` langsung bisa dipanggil telanjang. Tabrakan nama **lintas file** sekarang terdeteksi dan gagal kompilasi dengan pesan jelas, tapi tetap tidak ada isolasi/namespace sungguhan.

### Error runtime dari modul yang di-`muat` tidak menyebutkan nama file
```
Kesalahan Runtime: Baris 2: Tidak bisa membagi dengan nol.
```
Kalau error ini berasal dari fungsi di dalam file yang di-`muat` (bukan file utama), pesannya tetap cuma bilang "Baris 2" tanpa menyebutkan file mana. Ini beda dari error **kompilasi** (Lexer/Parser) untuk file yang di-`muat`, yang **sudah** menyebutkan nama file (`[nama_file.iso] Kesalahan Parser: ...`). Kalau proyek kamu punya banyak modul dan dapat error runtime dengan nomor baris yang ambigu, cek satu-satu file yang punya baris segitu.

### Registry paket (git-based, v1) — pin exact tag/rev saja, tidak ada version range
`isoteri tambah nama --git URL --tag vX.Y.Z` (atau `--rev <hash>`) mem-pin dependensi ke
SATU tag/commit persis. Tidak ada resolusi semver range (`^1.0`, `~2.3`, dst. seperti
Cargo/npm) — kalau upstream rilis versi baru, harus jalankan `isoteri tambah` ulang manual
dengan tag baru. Cache di `~/.isoteri/cache/` dianggap PIN: sekali ke-cache, tidak di-fetch
ulang lagi walau tag di remote dipindah ke commit lain (praktik buruk upstream, tapi bisa
terjadi) — hapus manual folder cache-nya (nama folder = URL+tag/rev yang disanitasi) kalau
itu terjadi dan kamu butuh isi terbaru. Belum ada index/discovery server (cara mencari
"paket apa saja yang tersedia") — harus tahu URL repo-nya sendiri. Butuh `git` terpasang &
ada di `PATH`.

---

## `ulang selaras` (Paralel)

Ini interpreter **terpisah dan jauh lebih terbatas** dari badan fungsi/`ulang` biasa, bukan sekadar "loop biasa yang diparalel":
- Item di daftar harus `Angka`/`Desimal`/`Teks`/`Bool` saja.
- Statement yang didukung cuma `ingat`, `tampilkan`, `kalau`/`lainnya`.
- **Tidak bisa memanggil fungsi apa pun** di dalam badannya (bawaan maupun buatan sendiri) — cuma literal, identifier, dan operator biner.

Lihat [REFERENSI.md](REFERENSI.md#ulang-selaras-paralel) untuk detail lengkap.

---

## `bentuk` (Struct)

### Representasi umum belum "JIT-able" — kecuali sebagai parameter fungsi
Instans `bentuk` yang disimpan di variabel biasa (`ingat x = Titik{...}`) tetap representasi immutable/clone-on-write biasa. **Kekecualian**: kalau sebuah `bentuk` semua field-nya numerik (`Angka`/`Desimal`) dan dipakai sebagai **tipe parameter fungsi**, parameternya otomatis "di-flatten" jadi slot langsung dan bisa ikut JIT — lihat [REFERENSI.md](REFERENSI.md#parameter-bentuk-yang-flattened). Berlaku juga untuk callback `petakan`/`saring`/`urutkan`. Batasannya:
- Tidak berlaku untuk closure/`PanggilNilai` (closure secara desain tidak pernah punya parameter yang di-flatten, jadi ini otomatis aman, bukan batasan yang perlu dikhawatirkan).
- Nama parameter itu sendiri **tidak bisa dipakai sebagai nilai utuh** di badan fungsi — cuma lewat `.field`.
- Fungsi **belum bisa mengembalikan** instans `bentuk` hasil JIT — nilai kembalian tetap `Angka`/`Desimal`.

### Field validasi terjadi saat kompilasi, bukan runtime
Ini sebenarnya keunggulan (error lebih awal, lebih jelas), tapi berarti kamu **tidak bisa** menangkap error field-kurang/field-asing lewat `coba/tangkap` — program gagal build sebelum sempat jalan sama sekali.

---

## Web Runtime (`isoteri ekspor-web` / `isoteri-vm.js`)

### Event handler -- sekarang menerima closure & data event
```
dom_ketika(tombol, "klik", fungsi() { tampilkan "diklik" })          catatan: 0 parameter, cara LAMA, tetap jalan
dom_ketika(input, "input", fungsi(e) { tampilkan e.nilai })          catatan: 1 parameter BARU -- baca data event
dom_ketika(tombol, "klik", "nama_fungsi")                             catatan: nama Teks, cara LAMA, tetap jalan
ingat ambang = 10
dom_ketika(tombol, "klik", fungsi(e) { kalau (hitung > ambang) {...} }) catatan: closure DENGAN capture juga bisa
```
`e` adalah instans `Event` dengan field: `tipe` (Teks, nama event mentah), `nilai` (Teks isi `.value` elemen target kalau ada, `Kosong` kalau tidak), `tombol` (Teks tombol keyboard yang ditekan kalau event keyboard, `Kosong` kalau bukan), `target` (`ElemenDOM`, buat dipakai lagi ke `dom_*` lain kalau perlu). Backward-compatible penuh: handler LAMA (0 parameter) terus dipanggil tanpa argumen persis seperti sebelumnya -- fungsi ini otomatis intip berapa parameter handler-nya sebelum manggil.

Fungsi baru buat form input: `dom_nilai(elemen)`/`dom_atur_nilai(elemen, teks)` (baca/tulis `.value`), `dom_dicentang(elemen)`/`dom_atur_dicentang(elemen, bool)` (checkbox), `dom_fokus(elemen)`.

### Timer -- `tunda()`/`interval_mulai()`/`interval_hentikan()`
```
tunda(1000, fungsi() { tampilkan "sedetik kemudian" })         catatan: setTimeout, sekali jalan
ingat id = interval_mulai(500, fungsi() { tampilkan "tik" })    catatan: setInterval, id buat berhenti nanti
interval_hentikan(id)
```
Callback timer terima 0 argumen (Teks nama fungsi ATAU closure, boleh dengan capture). `id` dari `interval_mulai()` itu `Angka` biasa yang bisa disimpan/dilewatkan sebagai variabel.

### Fetch lanjutan -- `unduh_lanjut_async()` (POST/header/status code)
```
unduh_lanjut_async(url, {"metode": "POST", "body": teks_json(data), "header": {"Content-Type": "application/json"}},
    fungsi(r) { tampilkan r.status; tampilkan r.ok; tampilkan urai_json(r.teks) },
    fungsi(pesan) { tampilkan "gagal: " + pesan })
```
`opsi` (Peta) semua kunci opsional: `metode` (default `"GET"`), `body` (Teks), `header` (Peta<Teks,Teks>). Callback sukses terima SATU argumen: instans `Respons` (`status`: Angka, `ok`: Bool, `teks`: Teks -- uraikan sendiri lewat `urai_json()` yang sudah ada kalau JSON). `unduh_async()` versi lama (GET-teks-doang) **tetap ada, tidak berubah**, sekarang juga menerima closure di kedua argumen fungsi-nya (bukan cuma Teks nama fungsi).

### Router -- `rute_daftar()`/`rute_mulai()`/`rute_navigasi()`/`rute_sekarang()` (hash routing)
```
fungsi render_beranda(params) { dom_atur_html(dom_pilih("#app"), "<h1>Beranda</h1>") }
fungsi render_produk(params) { tampilkan params["id"] }   catatan: dari pola "/produk/:id"

rute_daftar([
    {"pola": "/", "tampilkan": "render_beranda"},
    {"pola": "/produk/:id", "tampilkan": "render_produk"},
    {"pola": "*", "tampilkan": "render_404"}                catatan: catch-all/404, taruh PALING AKHIR
])
rute_mulai()               catatan: mulai dengarkan hashchange + cocokkan path saat ini
rute_navigasi("/produk/7") catatan: navigasi terprogram (mis. dari tombol)
rute_sekarang()             catatan: {path: Teks, params: Peta} rute aktif saat ini
```
URL berbentuk `situs.com/#/produk/7` (hash routing) -- **sengaja** dipilih ketimbang path routing (`situs.com/produk/7`) karena zero-config, langsung jalan di hosting statis apa pun (Vercel/Cloudflare Pages/GitHub Pages) tanpa perlu setting rewrite server. `:nama` menangkap satu segmen path, `*` di akhir jadi catch-all (isinya masuk `params["*"]`) -- cocok buat halaman 404. Query string (`#/cari?q=beras`) otomatis ke-parse gabung ke `params` yang sama. Handler terima Teks (nama fungsi) ATAU closure (dengan capture), persis konvensi `petakan`/`dom_ketika`/dst. **Batasan:** cuma satu level rute (belum ada nested routes/layout bertingkat) -- buat aplikasi kompleks, susun sendiri di dalam handler (mis. render_produk bisa manggil komponen anak sendiri).

### Manajemen state -- `state_buat()`/`state_nilai()`/`state_atur()`/`state_ubah()`/`state_langgan()`
```
ingat toko = state_buat(0)                                  catatan: nilai awal
state_langgan(toko, fungsi(n) { dom_atur_teks(el, ""+n) })  catatan: "pelanggan" (subscriber), langsung dipanggil sekali
state_atur(toko, 5)                                          catatan: set nilai baru -> SEMUA pelanggan dipanggil ulang
state_ubah(toko, fungsi(lama) { kembalikan lama + 1 })       catatan: update berbasis nilai lama (increment dst.)
state_nilai(toko)                                             catatan: baca nilai saat ini tanpa langganan
```
Pola **pub/sub sederhana** (bukan reactive fine-grained kayak Vue/Solid, bukan vdom-diffing kayak React) -- tiap `state_atur`/`state_ubah` memanggil ULANG SEMUA pelanggan dengan nilai baru PENUH; pelanggan (biasanya fungsi "render ulang" pakai `dom_atur_html`) tanggung jawab sendiri update tampilannya. Nilai yang disimpan bisa apa saja termasuk `bentuk`/`Daftar`/`Peta` bersarang (immutable/clone-on-write, konsisten dengan semantik bahasa). Cukup buat skala dashboard/CRUD/aplikasi warga -- **bukan** pengganti diffing DOM buat UI yang sangat besar & dalam (tiap render ulang, seluruh bagian yang di-`dom_atur_html` di-parse ulang browser dari nol, bukan di-patch sebagian).

### Performa: JS runtime TIDAK punya JIT -- jaga komputasi berat tetap di native
```
fungsi fib(n: Angka) { kalau (n <= 1) { kembalikan n } kembalikan fib(n-1) + fib(n-2) }
tampilkan fib(38)
```
Di native (`isoteri jalankan`), ini berkat JIT Cranelift selesai dalam hitungan detik. Di `isoteri-vm.js` (browser/Node) -- **diverifikasi langsung**, masih belum selesai setelah 90 detik, karena JS runtime murni interpretasi bytecode, tidak ada kompilasi native sama sekali. Ini bukan bug, tapi karakteristik arsitektur yang harus disadari: buat kalkulasi berat (rekursi dalam, loop jutaan iterasi), pertimbangkan lakukan di native Rust (mis. lewat API/Cloudflare Worker yang dipanggil dari web lewat `unduh_lanjut_async`) alih-alih langsung di `isoteri-vm.js`. UI logic biasa (routing, state, DOM manipulation, event handling) jauh lebih ringan dan tidak kena batasan ini.

### Component System -- `komponen_buat()`/`komponen_pasang()`/dst.
```
ingat todo = komponen_buat({
    "state_awal": TodoState { item_daftar: [], teks_input: "" },
    "render": fungsi(props, state) {
        kembalikan "<input data-aksi='ubah' data-peristiwa='input' value='" + state.teks_input + "'>" +
                   "<button data-aksi='tambah'>Tambah</button>"
    },
    "aksi": {
        "ubah": fungsi(props, state, e) { kembalikan TodoState { item_daftar: state.item_daftar, teks_input: e.nilai } },
        "tambah": fungsi(props, state, e) { catatan: ... kembalikan state_baru }
    },
    "dipasang": fungsi(props, state) { tampilkan "komponen siap" },      catatan: opsional, sekali pas mount
    "diperbarui": fungsi(props, state) { tampilkan "render ulang" },     catatan: opsional, tiap re-render
    "dilepas": fungsi(props, state) { tampilkan "dibongkar" }            catatan: opsional, pas komponen_lepas()
})
ingat instans = komponen_pasang(todo, dom_pilih("#app"), props_opsional)
komponen_state(instans)              catatan: baca state saat ini
komponen_atur_state(instans, nilai)  catatan: ganti state -> otomatis render ulang
komponen_ubah_state(instans, fungsi(lama) { kembalikan lama_diubah })
komponen_atur_props(instans, props_baru)
komponen_elemen(instans)             catatan: ElemenDOM wadah -- buat query manual kalau perlu
komponen_lepas(instans)              catatan: panggil "dilepas", copot listener, kosongkan wadah
```
**Filosofi (disengaja, bukan kelalaian):** ini pola **"render ulang penuh"**, BUKAN virtual-DOM diffing kayak React. `render` mengembalikan STRING HTML, ditulis langsung lewat `innerHTML` tiap state/props berubah. Cukup buat skala dashboard/CRUD/aplikasi warga -- bukan pengganti diffing sungguhan buat UI sangat besar & dalam (browser parse ulang HTML dari nol tiap render, bukan patch sebagian).

**Event lewat `data-aksi`, bukan `onclick=` inline:** karena `render` cuma menghasilkan teks HTML (bukan pointer fungsi hidup), gak ada cara nyuntik handler Isoteri langsung ke atribut `onclick`. Solusinya event delegation: tulis `data-aksi="nama"` di elemen HTML hasil render (opsional `data-peristiwa="input"`/`"change"`/`"submit"`/`"keyup"`, default `"click"`), lalu daftarkan handler yang sesuai lewat opsi `"aksi"` komponen. Handler aksi dapat `(props, state, event)`, **nilai kembaliannya JADI state baru** (pola reducer) -- otomatis memicu render ulang.

**Komposisi/nested components:** belum otomatis (belum ada children/slot bawaan). Pola yang jalan: `render` induk taruh placeholder `<div id='anak-1'></div>`, lalu di hook `"dipasang"`/`"diperbarui"` panggil `komponen_pasang()` manual buat tiap anak, target ke `dom_pilih("#anak-1")`.

### Yang masih belum ada
- **Component System** (React/Vue-style, dengan lifecycle hooks) -- BELUM dikerjakan, ini proyek besar tersendiri (butuh keputusan arsitektur: vdom-diffing vs render-ulang-penuh vs approach lain). Router + State Management di atas adalah FONDASI buat itu -- komponen nantinya = kombinasi state_buat() + fungsi render + rute_daftar(), tinggal disusun jadi pola/helper yang lebih rapi.
- **HTTP Interceptor** -- belum ada primitif bahasa baru, tapi BISA disusun sendiri sekarang lewat pola pembungkus: buat fungsi `unduh_dengan_auth(url, opsi, sukses, gagal)` yang nambahin header token lalu manggil `unduh_lanjut_async()` di dalamnya.
- Belum ada bridge clipboard (copy/paste).
- Belum ada akses ke `History`/routing SPA (`pushState` dst.).
- `dom_ketika()` belum bisa `removeEventListener` (sekali daftar, nempel selamanya sampai elemen dihapus).
- Semua penambahan di atas **cuma nyentuh `isoteri-vm.js` (JS murni)**, TIDAK nyentuh interpreter/VM/JIT Rust-nya sama sekali -- nol dampak ke performa jalur native, dan sudah diverifikasi lewat regresi 21 program contoh lewat `jalankan-node.js` (nol gagal, di luar limitasi `tulis_berkas()` yang memang sudah didokumentasikan gak berlaku di web).

---

## Penanganan Error

Hanya error **runtime** yang bisa ditangkap `coba/tangkap` (pembagian nol, indeks luar jangkauan, field tidak ditemukan, panggil nilai bukan-fungsi, dst.). Error **kompilasi** (tipe salah, variabel belum dideklarasikan, field `bentuk` kurang/asing, deklarasi ganda, dll.) terjadi sebelum program mulai jalan sama sekali, jadi tidak ada cara menangkapnya dari dalam kode Isoteri — program langsung berhenti dengan pesan `Kesalahan Kompilasi: ...` ke stderr.

---

## Tooling & Ekosistem

Tidak ada (belum dikerjakan sama sekali):
- **REPL** — tidak ada mode interaktif, cuma jalanin file `.iso`.
- **Debugger** — tidak ada breakpoint/step-through, cuma `coba/tangkap` dan `tampilkan` manual.
- **Test framework** bawaan bahasa — testing sejauh ini manual (jalanin file `.iso`, baca output).
- **Syntax highlighting** editor, **linter**, **LSP** — belum ada.
- **Automated test suite** untuk compiler/VM-nya sendiri — regression testing sejauh ini manual (jalanin semua `program*.iso` satu-satu, baca outputnya).

## Kompilasi Native & Platform

- **SIMD sempat dicoba** buat `jumlah()`/`rata_rata()` (AVX2), tapi **terbukti lebih lambat** dari versi scalar (~45% lebih lambat, diukur langsung) karena biaya "ekstraksi" nilai dari representasi `Value` yang tagged/boxed ke buffer mentah sama besarnya dengan biaya loop scalar itu sendiri — jadi direvert, bukan diship. Detail penyebabnya ada di [README.md](../README.md).
- **Target WebAssembly belum bisa dikerjakan di environment pengembangan ini** — bukan soal susah, tapi environment ini gak punya `rustup` (Rust-nya dari `apt`) dan gak ada akses jaringan ke `static.rust-lang.org` (tempat target `wasm32-unknown-unknown` biasanya didownload). Siapa pun yang mau lanjutkan ini butuh Rust yang terpasang lewat `rustup` (bukan `apt`) di mesinnya sendiri.
- **Kompilasi AOT (`isoteri bangun`) sudah ada**, tapi dengan batasan:
  - Butuh Rust & Cargo terpasang di mesin yang dipakai untuk **bangun** (bukan yang menjalankan hasilnya).
  - Deteksi `muat "..."` untuk bundling bersifat tekstual — statement `muat` harus sendirian di baris-nya.
  - Build pertama kali lambat (beberapa menit, kompilasi seluruh dependency dari nol); build berikutnya cepat berkat cache persisten, **selama** memakai mesin yang sama (cache tidak portable antar mesin).
  - Belum ada cross-compilation — hasil executable spesifik untuk platform tempat ia dibangun.
- Belum diuji di Windows.
- Build dari source **butuh Rust versi cukup baru**, atau pinning dependency manual di environment lama — lihat [INSTALASI.md](INSTALASI.md).
