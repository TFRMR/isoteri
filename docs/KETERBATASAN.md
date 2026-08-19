# Keterbatasan yang Diketahui

Dokumen ini mengumpulkan semua batasan bahasa Isoteri yang sudah teridentifikasi sampai saat ini — supaya tidak perlu ditemukan ulang dari nol, dan supaya pengguna tahu apa yang **memang belum didukung** vs **kemungkinan bug baru**. Semua yang tercantum di sini sudah diverifikasi lewat pengujian langsung, bukan dugaan.

Kalau kamu menemukan perilaku aneh yang **tidak** ada di daftar ini, kemungkinan itu bug baru, bukan batasan yang diketahui.

---

## Bahasa & Semantik

### Overflow `Angka` diam-diam wrap-around, bukan error
```
ingat besar = 9223372036854775807   catalog: i64::MAX
tampilkan besar + 1                  catatan: hasilnya -9223372036854775808, BUKAN error
```
Build `--release` Rust mematikan overflow-check secara default. Ini artinya kalkulasi yang melebihi jangkauan `i64` (~9.2 kuintiliun) akan menghasilkan angka yang salah secara diam-diam, bukan crash atau error yang bisa ditangkap. Kalau program kamu berpotensi menghasilkan angka sangat besar (mis. akumulasi token dalam satuan terkecil dalam jumlah masif), pertimbangkan pakai `Desimal` atau tambahkan validasi batas atas secara manual.

### Operator modulo (`%`), increment/decrement (`++`/`--`), compound assignment (`+=` dst.) -- didukung
```
tampilkan 17 % 5        catatan: 2
x += 1                    catatan: sama seperti x = x + 1
x++                       catatan: sama seperti x = x + 1 (HANYA statement baris sendiri, bukan ekspresi)
rek.saldo -= 30           catatan: compound assignment field juga didukung, termasuk nested
```
Semuanya gula sintaksis murni di parser (didesugar ke bentuk `nama = nama <op> nilai`), kecuali `%` yang jadi `BinOp` baru sungguhan (butuh entry di semua jalur: eval, formatter, tipe inferensi, IR, JSON export web). Modulo dengan pembagi 0 melempar error runtime jelas, sama seperti pembagian. `%` TIDAK di-JIT (sama seperti `/`, butuh cek pembagi-nol saat runtime) -- fungsi yang memakainya otomatis fallback ke bytecode VM biasa, tetap benar cuma lebih lambat dari fungsi murni aritmatika lain.

Efek samping yang perlu diketahui: karena `+=`/`++`/`--` didesugar TOTAL tanpa jejak di AST, `isoteri format` akan menormalisasi balik ke bentuk eksplisit (`total += 5` -> `total = total + 5`) -- ini bukan bug, cuma gula sintaksisnya memang tidak "diingat" formatter. `++`/`--` cuma didukung buat variabel (`i++`), belum buat field (`objek.field++`).

### `putus`/`lanjut` (break/continue) -- didukung di eksekusi normal, belum di `via-ir`/AOT
Sudah bisa dipakai di `ulang` dan `ulang setiap` (loop terdekat, boleh bersarang, aman dipakai di dalam `coba/tangkap`):
```
ulang (i < 10) {
    i = i + 1
    kalau (i == 3) { lanjut }   catatan: lompat ke iterasi berikutnya
    kalau (i == 7) { putus }    catatan: keluar loop
    tampilkan i
}
```
Belum bisa dipakai lewat `isoteri via-ir` atau `isoteri bangun` (AOT) -- keduanya lewat jalur IR terpisah yang belum diimplementasikan buat dua statement ini, akan panik dengan pesan jelas kalau dicoba. Pakai `isoteri jalankan` (mode biasa) untuk sekarang. `ulang selaras` (loop paralel) juga belum mendukung `putus`/`lanjut` -- evaluatornya memang sudah dibatasi terpisah.

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

### Deklarasi ulang `ingat` dengan nama sama (di file yang sama) diterima diam-diam
```
ingat x = 5
ingat x = 10     catatan: TIDAK error -- x sekarang 10, deklarasi pertama "hilang"
```
Ini beda dari duplikasi nama `fungsi`/parameter/field `bentuk` (yang sekarang sudah di-cek dan gagal kompilasi) — untuk `ingat`, redeklarasi nama sama di file yang sama sengaja dibiarkan seperti semula, karena mengubahnya berisiko mematahkan pola kode yang sudah ada (mis. reset nilai variabel loop-like) tanpa manfaat yang jelas sepadan.

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
