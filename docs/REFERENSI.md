# Referensi Bahasa Isoteri

Dokumen ini adalah rujukan lengkap sintaks dan fitur bahasa Isoteri. Untuk pengantar langkah-demi-langkah, baca [TUTORIAL.md](TUTORIAL.md) dulu. Untuk batasan yang perlu diketahui, lihat [KETERBATASAN.md](KETERBATASAN.md).

---

## Daftar Isi

1. [Komentar](#komentar)
2. [Tipe Data](#tipe-data)
3. [Variabel](#variabel)
4. [Operator](#operator)
5. [Percabangan](#percabangan)
6. [Perulangan](#perulangan)
7. [Fungsi](#fungsi)
8. [Closure (Fungsi Anonim)](#closure-fungsi-anonim)
9. [Bentuk (Struct/Tipe Custom)](#bentuk-structtipe-custom)
10. [Daftar (List)](#daftar-list)
11. [Peta (Dictionary)](#peta-dictionary)
12. [Penanganan Error](#penanganan-error)
13. [Modul (`muat`)](#modul-muat)
14. [Fungsi Bawaan (Standard Library)](#fungsi-bawaan-standard-library)
15. [Kompilasi JIT](#kompilasi-jit)
    - [Parameter Bentuk yang "Flattened"](#parameter-bentuk-yang-flattened)
16. [Kompilasi AOT (Executable Mandiri)](#kompilasi-aot-executable-mandiri)

---

## Komentar

```
catatan: ini komentar satu baris, sampai akhir baris
```

Tidak ada komentar multi-baris. Setiap baris komentar harus diawali `catatan:`.

---

## Tipe Data

| Tipe | Contoh literal | Keterangan |
|---|---|---|
| `Angka` | `42`, `-7`, `0` | Integer bertanda, 64-bit (`i64`) |
| `Desimal` | `3.14`, `-0.5`, `2.0` | Floating point 64-bit (`f64`) |
| `Teks` | `"halo dunia"` | String. Escape yang didukung: `\"`, `\n`, `\\` |
| `Bool` | `benar`, `salah` | Boolean |
| `Daftar` | `[1, 2, 3]` | List dinamis, boleh campur tipe |
| `Peta` | `{"kunci": nilai}` | Dictionary, kunci harus literal Teks |
| `Bentuk` (instans) | `Petani { nama: "Budi" }` | Lihat bagian [Bentuk](#bentuk-structtipe-custom) |
| `Fungsi` | `fungsi(n) { kembalikan n }` | Nilai closure, lihat bagian [Closure](#closure-fungsi-anonim) |
| `kosong` | `kosong` | Nilai kosong/null |

Tidak ada konversi tipe implisit antara `Angka` dan `Desimal` dalam operasi campuran kecuali lewat fungsi eksplisit (`ke_desimal`, `ke_bulat`) — tapi literal `Angka` di dalam badan closure/fungsi bertipe `Desimal` otomatis dipromosikan ke konstanta `Desimal` saat kompilasi (lihat [Kompilasi JIT](#kompilasi-jit)).

---

## Variabel

```
ingat nama = "Budi"                 catatan: deklarasi, tipe disimpulkan
ingat umur: Angka = 25              catatan: deklarasi dengan anotasi tipe eksplisit
simpan kota = "Kulon Progo"         catatan: 'simpan' adalah sinonim dari 'ingat'

nama = "Siti"                       catatan: pengubahan (assignment) -- TANPA 'ingat'/'simpan'
```

- `ingat` dan `simpan` adalah kata kunci yang identik (sinonim), pilih salah satu sesuai selera.
- **`ingat`/`simpan` nama yang sama dua kali di scope yang sama sekarang gagal kompilasi** dengan pesan jelas (dulu diam-diam menimpa nilai lama). Kalau memang mau MENGUBAH nilai variabel yang sudah ada, pakai `nama = nilai_baru` (tanpa `ingat`/`simpan`) — itu tetap sah.
- Assignment ulang (`nama = ...` tanpa `ingat`) mengharuskan variabelnya **sudah** dideklarasikan sebelumnya.
- Anotasi tipe (`: Angka`, `: Desimal`, dst.) bersifat opsional tapi divalidasi saat kompilasi kalau dicantumkan — memberi nilai bertipe salah ke variabel yang dianotasi akan gagal kompilasi, bukan gagal saat program jalan.
- **Variabel global harus dideklarasikan sebelum dipakai secara tekstual** — tidak ada forward-reference untuk `ingat` (beda dengan `fungsi` dan `bentuk`, yang boleh dipakai sebelum dideklarasikan di tempat lain dalam file yang sama).

### Assignment field bersarang

```
budi.alamat.desa = "Purwosari"      catatan: rantai '.field' boleh sedalam apapun
```

Lihat [Bentuk](#bentuk-structtipe-custom) untuk detail.

---

## Operator

| Kategori | Operator |
|---|---|
| Aritmatika | `+` `-` `*` `/` `%` |
| Perbandingan | `==` `!=` `>` `>=` `<` `<=` |
| Logika | `dan` `atau` `!` (negasi unary) |
| Compound assignment | `+=` `-=` `*=` `/=` |
| Increment/decrement | `++` `--` (statement baris sendiri saja, bukan ekspresi) |
| Akses field | `.` (mis. `budi.nama`) |
| Indeks | `[...]` (mis. `daftar[0]`, `peta["kunci"]`) — bisa juga di sisi kiri assignment (lihat bawah) |

`+` pada `Teks` melakukan penggabungan string, dan otomatis mengonversi operand non-Teks (Angka, Desimal, Bool, dst.) ke representasi Teks-nya — jadi `"Total: " + 5` menghasilkan `"Total: 5"`.

`!ekspr` pakai *truthiness* yang sama dengan kondisi `kalau`/`dan`/`atau` (lihat `Value::truthy()`): `Bool` apa adanya, `Angka`/`Desimal` nol itu salah, `Teks`/`Daftar`/`Peta` kosong itu salah, `Kosong` selalu salah, selain itu benar. Jadi `!5` → `salah`, `!0` → `benar`, `!""` → `benar`.

`+=`/`-=`/`*=`/`/=` dan `++`/`--` murni gula sintaksis (`x += 1` sama dengan `x = x + 1`) — berlaku juga buat field (`objek.saldo += 100`) dan indeks (`daftar[0] += 1`).

Assignment lewat indeks: `daftar[0] = 99`, `peta["kunci"] = 99`, boleh nested (`matriks[0][1] = x`) dan campur dengan field (`objek.daftar[0] = x`). `Peta`: kunci yang belum ada otomatis ditambahkan. `Daftar`: indeks harus sudah ada (di luar jangkauan → error, tidak auto-extend, pakai `tambah()` buat menambah elemen).

Overflow `Angka` (di luar jangkauan `i64`) menghasilkan error runtime jelas di eksekusi normal — lihat `docs/KETERBATASAN.md` untuk pengecualian jalur JIT.

---

## Percabangan

```
kalau (kondisi) {
    ...
} lainnya kalau (kondisi_lain) {
    ...
} lainnya {
    ...
}
```

- `jika` adalah sinonim dari `kalau`.
- Kondisi **wajib** dikurung `(...)`.
- `lainnya kalau` (else-if) boleh dirantai berapa kali pun, gula sintaksis murni (desugar jadi `kalau` bersarang di dalam `lainnya`).
- Blok `lainnya` opsional.

---

## Perulangan

### `ulang` (while)

```
ulang (kondisi) {
    ...
}
```

`putus` keluar paksa dari loop (break), `lanjut` lompat ke iterasi berikutnya (continue) — dua-duanya boleh dipakai di dalam `ulang`/`ulang setiap`, boleh bersarang (selalu ke loop terdekat), dan aman dipakai di dalam `coba/tangkap`. Error kompilasi jelas kalau dipakai di luar loop.

### `ulang setiap` (foreach)

```
ulang setiap item dari daftar {
    tampilkan item
}
```

Bekerja untuk `Daftar`.

### `ulang selaras` (paralel)

```
ulang selaras setiap item dari daftar {
    ...
}
```

Sintaksnya identik dengan `ulang setiap`, tapi tiap iterasi dijalankan **paralel** di thread terpisah (sebanyak core CPU yang tersedia), dengan output `tampilkan` dikumpulkan lalu dicetak di akhir sesuai urutan asli daftar (supaya hasilnya tetap deterministik meski eksekusinya paralel).

**Ini interpreter terpisah yang jauh lebih terbatas** dari badan fungsi/`ulang` biasa — bukan sekadar "loop biasa yang diparalel". Batasannya:
- Item di dalam `daftar` harus `Angka`, `Desimal`, `Teks`, atau `Bool` — bukan `Bentuk`/`Daftar`/`Peta`/`Fungsi`.
- Statement yang didukung di badannya **cuma**: `ingat`, `tampilkan`, `kalau`/`lainnya`. Tidak ada `ulang` bersarang, tidak ada `kembalikan`, tidak ada pengubahan field, tidak ada `muat`.
- Ekspresi yang didukung **cuma**: literal, identifier, dan operator biner (`+ - * / == != > >= < <= dan atau`). **Tidak bisa memanggil fungsi apa pun** (baik fungsi bawaan seperti `akar()` maupun fungsi buatan sendiri) di dalam badan `ulang selaras` — kalau butuh logika lebih kompleks per-item, olah dulu di luar loop atau pakai `petakan()`/`ulang setiap` biasa (non-paralel).

Tidak ada `putus`/`lanjut` (break/continue) di bahasa ini, baik di loop biasa maupun `ulang selaras`.

---

## Fungsi

```
fungsi nama_fungsi(param1: Tipe1, param2: Tipe2) {
    ...
    kembalikan nilai
}
```

- Anotasi tipe parameter opsional. Kalau dicantumkan, dipakai juga untuk menentukan apakah fungsi eligible dikompilasi JIT (lihat [Kompilasi JIT](#kompilasi-jit)).
- Fungsi **hanya boleh dideklarasikan di level atas program** (tidak bisa nested di dalam fungsi/`kalau`/`ulang` lain sebagai `fungsi nama(...)` biasa — untuk fungsi nested/anonim, pakai [closure](#closure-fungsi-anonim)).
- Fungsi boleh dipanggil sebelum dideklarasikan secara tekstual (forward-reference didukung, beda dengan variabel).
- Nama fungsi harus unik dalam satu program gabungan (lintas file lewat `muat` termasuk) — deklarasi ganda gagal kompilasi dengan pesan jelas.
- **Fungsi bisa membaca variabel global** (dibaca *live*, bukan snapshot) selama variabel globalnya dideklarasikan lebih dulu secara tekstual sebelum fungsi tersebut.
- Nama parameter harus unik dalam satu fungsi — `fungsi f(a, a)` gagal kompilasi.

---

## Closure (Fungsi Anonim)

```
ingat kuadrat = fungsi(n) {
    kembalikan n * n
}
tampilkan kuadrat(5)          catatan: 25
```

- Closure adalah **ekspresi** (`fungsi(params) { badan }`), bukan statement — bisa dipakai di mana pun ekspresi diterima: ditugaskan ke variabel, dilewatkan sebagai argumen, disimpan di `Daftar`, dikembalikan dari fungsi lain.
- Closure **menangkap** (capture) variabel dari scope pembungkusnya:
  ```
  fungsi buat_penambah(tambahan) {
      kembalikan fungsi(n) { kembalikan n + tambahan }
  }
  ingat tambah5 = buat_penambah(5)
  tambah5(10)      catatan: 15
  ```
- **Penting: capture itu snapshot NILAI, bukan referensi hidup.** Kalau variabel yang ditangkap berubah setelah closure-nya dibuat, closure-nya tidak ikut berubah. Ini beda dari kebanyakan bahasa (JS, Python) yang pakai referensi.
- Closure bisa nested (closure di dalam closure) dan capture-nya transitif — closure paling dalam bisa menangkap variabel dari *kakek*-scope-nya, bukan cuma induk langsung.
- Closure di level atas yang ditugaskan lewat `ingat nama = fungsi(...) {...}` **bisa** rekursi ke dirinya sendiri lewat namanya. Closure yang di-nested di dalam fungsi lain **tidak bisa** — lihat [KETERBATASAN.md](KETERBATASAN.md).
- Memanggil sebuah variabel (`f(x)`) otomatis dideteksi sebagai "panggil nilai closure" kalau `f` adalah variabel, atau "panggil fungsi statis" kalau `f` adalah nama fungsi yang dikenal. Kalau ada variabel DAN fungsi bernama sama, variabel menang (aturan shadowing).

---

## Bentuk (Struct/Tipe Custom)

### Deklarasi

```
bentuk Petani {
    nama: Teks,
    lahan: Angka,
    hasil: Desimal
}
```

- Semua field **wajib** punya anotasi tipe.
- Boleh dideklarasikan di mana saja di level atas file (forward-reference didukung, boleh dipakai sebelum baris deklarasinya).
- Nama field harus unik dalam satu `bentuk` — duplikat gagal kompilasi.

### Membuat instans

```
ingat budi = Petani { nama: "Budi", lahan: 2, hasil: 15.5 }
```

- Urutan field di literal **bebas** (tidak harus sama dengan urutan deklarasi).
- **Semua field wajib diisi** — field yang kurang, field asing (tidak ada di skema), field duplikat, atau tipe nilai yang salah semuanya gagal **saat kompilasi** (bukan saat program jalan), dengan pesan error yang menyebutkan nama field/bentuk yang bermasalah.

### Akses & pengubahan field

```
tampilkan budi.nama              catatan: baca field
budi.hasil = 20.0                 catatan: ubah field (1 level)
budi.alamat.desa = "Purwosari"    catatan: ubah field bersarang (berapa pun level-nya)
```

Field bersarang (`bentuk` di dalam `bentuk`) didukung penuh untuk baca maupun tulis.

### Catatan implementasi

Representasi instans `Bentuk` bersifat immutable/clone-on-write (mirip `Peta`) — setiap `ubah field` sebenarnya membuat instans baru di belakang layar, bukan mengubah memori di tempat. Ini konsisten dengan gaya bahasa secara keseluruhan.

**Pengecualian buat performa**: kalau semua field sebuah `bentuk` bertipe numerik (`Angka`/`Desimal`), dan `bentuk` itu dipakai sebagai **tipe parameter** sebuah fungsi, parameternya otomatis "di-flatten" — field-nya disimpan sebagai slot lokal langsung (bukan `Vec` dinamis), dan bisa ikut dikompilasi JIT. Lihat [Kompilasi JIT: Parameter Bentuk yang "Flattened"](#parameter-bentuk-yang-flattened) untuk detail & batasannya.

---

## Daftar (List)

```
ingat harga = [5000, 7500, 6200]

harga[0]                    catatan: indeks baca, dimulai dari 0
panjang(harga)               catatan: 3
gabung(harga, 9999)          catatan: kembalikan Daftar BARU dengan item ditambahkan di akhir
jumlah(harga)                 catatan: total semua elemen numerik
rata_rata(harga)              catatan: rata-rata elemen numerik
ambil(harga, 1)               catatan: sama seperti harga[1], tapi lewat fungsi
```

Assignment lewat indeks didukung: `daftar[0] = x` mengubah elemen yang sudah ada (indeks di luar jangkauan → error, tidak auto-extend). Buat menambah elemen baru, tetap pakai `gabung()`.

### Fungsi list lanjutan (map/filter/sort)

Sintaks:

```
fungsi kuadrat(n) { kembalikan n * n }
fungsi genap(n) { kembalikan n % 2 == 0 }

petakan(daftar, "kuadrat")                     catatan: map -- nama fungsi via Teks (cara klasik)
petakan(daftar, fungsi(n) { kembalikan n*n })  catatan: map -- closure inline langsung
saring(daftar, genap)                            catatan: filter -- closure lewat variabel, harus kembalikan Bool
urutkan(daftar)                                   catatan: sort natural (Angka/Desimal/Teks)
urutkan(daftar, "nama_fungsi")                   catatan: sort berdasarkan kunci hasil fungsi (mis. field bentuk)

ingat ambang = 10
saring(daftar, fungsi(n) { kembalikan n > ambang })  catatan: closure DENGAN capture juga bisa
```

Argumen kedua ketiga fungsi ini terima **Teks** (nama fungsi) **ATAU closure first-class** sekaligus — kalau closure-nya punya *capture* (menangkap variabel dari scope luar), itu otomatis disambung transparan di belakang layar. Yang **belum** bisa: melewatkan nama fungsi top-level TANPA tanda kutip sebagai nilai (`petakan(d, kuadrat)` tanpa closure literal/string gagal, karena fungsi top-level bukan first-class value otomatis) — bungkus jadi closure kecil kalau perlu: `fungsi(x) { kembalikan kuadrat(x) }`.

Fungsi callback di `petakan`/`saring`/`urutkan` **tetap dapat manfaat JIT** kalau fungsinya eligible (parameter & lokal-nya seragam Angka/Desimal), baik dipanggil lewat nama string maupun closure. Ini termasuk fungsi dengan parameter `bentuk` yang di-flatten (lihat [Parameter Bentuk yang "Flattened"](#parameter-bentuk-yang-flattened)) — jadi `urutkan(daftar_titik, "jarak_dari_pusat")` bekerja dan tetap cepat, bukan cuma untuk parameter `Angka`/`Desimal` polos.

---

## Peta (Dictionary)

```
ingat profil = {"nama": "Budi", "umur": 25}

profil["nama"]              catatan: akses lewat kunci (harus Teks)
panjang(profil)              catatan: jumlah pasangan kunci-nilai
kunci_peta(profil)           catatan: Daftar berisi semua kunci (Teks)
```

Kunci literal Peta harus `Teks` diapit tanda kutip (tidak ada shorthand seperti `{nama: "Budi"}`  tanpa kutip — itu justru sintaks literal `Bentuk`, beda makna).

Assignment lewat kunci didukung: `peta["x"] = y` — kunci yang belum ada otomatis ditambahkan (insert-or-update), beda dari `Daftar` yang harus indeksnya sudah ada.

---

## Penanganan Error

```
coba {
    ingat hasil = 10 / pembagi
} tangkap pesan {
    tampilkan "Error: " + pesan
}
```

- `pesan` (variabel di `tangkap`) berisi `Teks` deskripsi error, biasanya diawali `"Baris N: ..."`.
- Hanya error **runtime** yang bisa ditangkap (pembagian nol, indeks di luar jangkauan, field tidak ditemukan, dst.). Error **kompilasi** (tipe salah, variabel tidak dideklarasikan, field `bentuk` kurang, dst.) tidak bisa ditangkap `coba/tangkap` karena terjadi sebelum program mulai dijalankan sama sekali.

---

## Modul (`muat`)

```
muat "matematika.iso"
muat "sub_folder/petani.iso"
muat "petani.iso" sebagai petani
```

- `muat` mengekspansi (menempel isi file lain) **sebelum** kompilasi — bukan fitur runtime.
- Path relatif dihitung dari lokasi file yang menulis `muat`-nya (bukan selalu dari file utama) — jadi modul yang di-`muat` boleh `muat` modul lain relatif ke dirinya sendiri.
- Memuat file yang sama dua kali (langsung atau lewat siklus) otomatis dilewati di kunjungan kedua (include guard) — aman, tidak jadi dobel-definisi.
- `muat` hanya boleh dipakai di level atas program (bukan di dalam `fungsi`/`kalau`/`ulang`).

**Tanpa `sebagai` (gaya lama, tetap didukung penuh)**: semua nama (fungsi/bentuk/variabel
global) tetap satu ruang nama global — tidak ada prefix per modul, cukup `kuadrat()` langsung
setelah `muat`. **Nama yang sama dideklarasikan di DUA FILE BERBEDA akan gagal kompilasi**
dengan pesan yang menyebutkan kedua file itu. Duplikasi nama dalam **satu file yang sama** tidak
diperiksa aturan ini (lihat aturan duplikat fungsi/parameter/field di atas, yang berlaku
terpisah).

**Dengan `sebagai alias`**: fungsi top-level modul itu diakses lewat `alias.nama(...)`, TIDAK
numplek ke namespace global — jadi dua modul independen boleh punya fungsi dengan nama yang
sama persis tanpa bentrok:
```isoteri
muat "petani.iso" sebagai petani
muat "toko.iso" sebagai toko

tampilkan petani.hitung(3, 4)   catatan: fungsi hitung() milik petani.iso
tampilkan toko.hitung(3, 4)     catatan: fungsi hitung() milik toko.iso -- BEDA fungsi, gak bentrok
```
Batasan yang perlu diketahui:
- Baru mencakup **fungsi**. Akses `bentuk`/variabel global lewat alias (mis. `petani.Petani { ... }`) belum didukung — modul yang diakses lewat alias sebaiknya cuma isi fungsi murni untuk saat ini.
- Fungsi di dalam modul beralias tetap bisa saling panggil satu sama lain seperti biasa (tanpa perlu prefix alias) — alias cuma berlaku dari LUAR modul itu.
- Alias yang sama tidak boleh dipakai dua kali di file yang sama (error jelas kalau dilanggar).
- `x.y(args)` di mana `x` BUKAN alias modul yang dikenal berarti "panggil NILAI di field `y` milik `x`" (mis. closure yang disimpan sebagai field bentuk) — bukan error, tapi kemampuan terpisah yang kebetulan pakai sintaks sama.

---

## Fungsi Bawaan (Standard Library)

### Daftar & Peta

| Fungsi | Signature | Keterangan |
|---|---|---|
| `panjang(x)` | `Daftar\|Teks\|Peta -> Angka` | Jumlah elemen/karakter/pasangan |
| `gabung(daftar, item)` | `Daftar, Nilai -> Daftar` | Daftar baru dengan `item` ditambahkan di akhir |
| `ambil(struktur, kunci)` | `Daftar, Angka -> Nilai` atau `Peta, Teks -> Nilai` | Sama seperti `[...]` |
| `jumlah(daftar)` | `Daftar -> Angka\|Desimal` | Total elemen numerik |
| `rata_rata(daftar)` | `Daftar -> Desimal` | Rata-rata elemen numerik |
| `kunci_peta(peta)` | `Peta -> Daftar` | Semua kunci sebagai Daftar Teks |
| `petakan(daftar, "fn")` | `Daftar, Teks -> Daftar` | Map |
| `saring(daftar, "fn")` | `Daftar, Teks -> Daftar` | Filter (fn kembalikan Bool) |
| `urutkan(daftar)` | `Daftar -> Daftar` | Sort natural |
| `urutkan(daftar, "fn")` | `Daftar, Teks -> Daftar` | Sort berdasar kunci hasil fn |

### Matematika

| Fungsi | Signature | Keterangan |
|---|---|---|
| `akar(x)` | `Angka\|Desimal -> Desimal` | Akar kuadrat (error kalau `x < 0`) |
| `pangkat(basis, eksponen)` | `-> Angka` (kalau keduanya Angka, eksponen ≥ 0) atau `Desimal` | Pemangkatan |
| `bulat(x)` | `-> Angka` | Pembulatan terdekat |
| `bulat_bawah(x)` | `-> Angka` | Floor |
| `bulat_atas(x)` | `-> Angka` | Ceil |
| `mutlak(x)` | `-> Angka\|Desimal` (mengikuti tipe input) | Nilai absolut |
| `min(a, b)` | `-> Angka\|Desimal` | Nilai lebih kecil |
| `maks(a, b)` | `-> Angka\|Desimal` | Nilai lebih besar |
| `acak()` | `-> Desimal` | Angka acak di `[0, 1)` (xorshift64, bukan kriptografis) |

### Teks

| Fungsi | Signature | Keterangan |
|---|---|---|
| `potong(teks, mulai, akhir)` | `Teks, Angka, Angka -> Teks` | Substring berbasis indeks karakter (bukan byte) |
| `ganti(teks, dari, ke)` | `Teks, Teks, Teks -> Teks` | Ganti semua kemunculan |
| `huruf_besar(teks)` | `Teks -> Teks` | Uppercase |
| `huruf_kecil(teks)` | `Teks -> Teks` | Lowercase |
| `pangkas(teks)` | `Teks -> Teks` | Trim whitespace kanan-kiri |
| `pisah(teks, pemisah)` | `Teks, Teks -> Daftar` | Split jadi Daftar Teks |
| `satukan(daftar, pemisah)` | `Daftar, Teks -> Teks` | Join Daftar Teks jadi satu Teks |
| `mengandung(teks, sub)` | `Teks, Teks -> Bool` | Contains |
| `diawali(teks, awalan)` | `Teks, Teks -> Bool` | Starts with |
| `diakhiri(teks, akhiran)` | `Teks, Teks -> Bool` | Ends with |

### Konversi Tipe

| Fungsi | Signature | Keterangan |
|---|---|---|
| `ke_desimal(x)` | `Angka\|Desimal -> Desimal` | |
| `ke_angka(x)` | `Angka\|Desimal\|Teks -> Angka` | Dari Teks: parse integer (i64) murni, TANPA titik/notasi ilmiah -- error jelas kalau formatnya bukan integer valid (mis. `"12.5"`) |
| `ke_bulat(x)` | `Angka\|Desimal -> Angka` | Truncate, bukan round |
| `ke_teks(x)` | `Nilai apapun -> Teks` | Sama seperti representasi `Display` |

### JSON

| Fungsi | Signature | Keterangan |
|---|---|---|
| `urai_json(teks)` | `Teks -> Nilai` | Parse string JSON jadi nilai Isoteri |
| `teks_json(nilai)` | `Nilai -> Teks` | Serialize nilai Isoteri jadi string JSON |

### Berkas & Jaringan

| Fungsi | Signature | Keterangan |
|---|---|---|
| `baca_berkas(path)` | `Teks -> Teks` | Baca isi file sebagai Teks |
| `tulis_berkas(path, isi)` | `Teks, Teks -> Bool` | Tulis Teks ke file |
| `unduh(url)` | `Teks -> Teks` | HTTP GET, kembalikan body sebagai Teks |

### HTTP Server (`server_mulai`)

**Cuma native (CLI, `isoteri bangun`), TIDAK tersedia di browser/wasm32**
(browser tidak bisa buka listening socket TCP -- batasan platform, bukan
batasan Isoteri). Blocking secara sengaja (konsisten dengan model eksekusi
VM yang sinkron, sama seperti `unduh()`) -- TIDAK ada runtime async.

Ini prasyarat inti buat pola **"satu skema, dua sisi"** (lihat bagian "Arah
strategis" di `ROADMAP.md`): `bentuk` + fungsi validasi yang sama bisa
`muat` dari backend (di sini) DAN frontend (browser, lewat `ekspor-web`)
tanpa duplikasi/drift antara keduanya.

```isoteri
fungsi tangani(req) {
    kalau (req["path"] == "/") {
        kembalikan "Halo!"                       catatan: Teks -> 200, text/plain
    }
    kalau (req["path"] == "/api/petani") {
        kembalikan {"nama": "Budi", "lahan": 2}   catatan: Peta -> 200, application/json (otomatis)
    }
    kembalikan respons_status(404, "Tidak ditemukan")   catatan: status custom
}

server_mulai(8899, "tangani")   catatan: blocking -- program "berhenti" di sini, layani request selamanya
```

| Fungsi | Signature | Keterangan |
|---|---|---|
| `server_mulai(port, handler)` | `Angka, (Teks \| Fungsi) -> Kosong` | Buka HTTP server di `port`, blocking selamanya (Ctrl+C buat berhenti). `handler` gaya sama seperti callback `petakan`/`urutkan` -- boleh nama fungsi (Teks) atau closure 1 parameter. |
| `respons_status(kode, nilai)` | `Angka, Nilai -> Instans` | Bungkus `nilai` supaya respons pakai kode status `kode` (100-599) alih-alih default 200. `nilai`-nya sendiri tetap ikut aturan konversi biasa (Teks/Peta/dst) di bawah. |

**Argumen `req` yang diterima `handler`** -- sebuah `Peta` (akses lewat
`req["..."]`, BUKAN `req....` -- itu cuma berlaku buat `bentuk`):

| Kunci | Tipe | Isi |
|---|---|---|
| `"metode"` | Teks | `"GET"`, `"POST"`, dst. |
| `"path"` | Teks | Path URL, TANPA query string (mis. `/api/petani`). |
| `"query"` | Peta | Parameter query string (`?a=1&b=2` -> `{"a": "1", "b": "2"}`), semua nilai Teks. |
| `"header"` | Peta | Header request, nama header -> nilai (Teks). |
| `"body"` | Teks | Isi body mentah (kosong untuk GET biasa). |

**Nilai balik `handler` diinterpretasi otomatis**:

| Tipe nilai balik | Status | Content-Type | Body |
|---|---|---|---|
| `Teks` | 200 | `text/plain` | Teks itu apa adanya |
| `Peta` / `Daftar` / `Instans` / `Angka` / dst | 200 | `application/json` | Di-serialize otomatis lewat mesin JSON yang sama dipakai `tulis_berkas()` |
| `Kosong` | 204 | - | kosong |
| hasil `respons_status(kode, nilai)` | `kode` | ikut aturan `nilai` di atas | ikut aturan `nilai` di atas |

Error yang terjadi DI DALAM `handler` (mis. field yang salah diakses) TIDAK
menghentikan server -- request itu dibalas 500 dengan pesan error sebagai
JSON, server tetap jalan melayani request berikutnya.

---

## Kompilasi JIT

Fungsi (termasuk closure tanpa capture) yang memenuhi **semua** syarat berikut otomatis dikompilasi jadi kode mesin asli lewat Cranelift, bukan dieksekusi lewat bytecode VM:

1. Punya minimal 1 parameter.
2. **Semua** parameter dan variabel lokal bertipe seragam — semuanya `Angka` ATAU semuanya `Desimal` (tidak campur, tidak ada yang tak-bertipe).
3. Badan fungsi hanya berisi: aritmatika `+` `-` `*` (bukan `/`), perbandingan, `dan`/`atau`, `kalau`/`ulang`, dan panggilan rekursif ke dirinya sendiri dengan arity yang cocok.
4. Tidak mengakses variabel global, tidak memanggil fungsi lain (selain dirinya sendiri), tidak menyentuh `Teks`/`Bool`/`Daftar`/`Peta`/`Bentuk`.
5. Untuk closure: tidak menangkap variabel apa pun dari scope pembungkus (closure dengan capture selalu jalan lewat bytecode).

Kalau salah satu syarat tidak terpenuhi, fungsi tetap jalan normal lewat bytecode VM — tidak ada error, cuma tidak dapat percepatan JIT. Tidak perlu anotasi manual apa pun untuk memicu ini; kompilator mendeteksinya otomatis.

### Parameter Bentuk yang "Flattened"

```
bentuk Titik { x: Angka, y: Angka }

fungsi jarak_kuadrat(p: Titik) {
    kembalikan p.x * p.x + p.y * p.y
}
```

Kalau parameter fungsi bertipe sebuah `bentuk` yang **semua field-nya numerik** (`Angka`/`Desimal`, boleh campur keduanya), Isoteri otomatis "meratakan" (flatten) parameter itu jadi beberapa slot lokal langsung (satu slot per field) alih-alih satu instans opak. Ini membuka jalan supaya field-nya bisa diakses tanpa lookup dinamis — dan kalau semua field-nya bertipe SAMA (semua `Angka` atau semua `Desimal`, syarat JIT biasa), fungsinya jadi eligible dikompilasi JIT juga.

**Batasan penting yang perlu diketahui:**
- **Cuma jalan lewat panggilan nama fungsi statis langsung, ATAU lewat `petakan`/`saring`/`urutkan`** (keduanya sudah mendukung parameter yang di-flatten) — tidak berlaku untuk closure (closure secara desain tidak pernah punya parameter yang di-flatten) atau `PanggilNilai` (memanggil lewat variabel).
- Argumen di posisi parameter yang di-flatten **boleh berupa ekspresi apa pun** (variabel, panggilan fungsi lain, literal `bentuk` langsung) — ekspresinya dijamin cuma dievaluasi **sekali**, aman dipakai meski ada efek samping.
- **Nama parameter itu sendiri tidak bisa dipakai sebagai nilai utuh** di dalam badan fungsi — cuma `p.field`, bukan `p` telanjang (misalnya buat dilewatkan ke fungsi lain atau disimpan ke variabel lain). Ini dideteksi saat kompilasi dengan pesan error yang jelas.
- **Fungsi belum bisa mengembalikan instans `bentuk`** hasil dari JIT — nilai kembaliannya tetap harus `Angka`/`Desimal` biasa.
- Kalau salah satu field `bentuk`-nya bukan numerik (misalnya ada `Teks`), parameter itu **tidak** di-flatten — tetap jadi instans `bentuk` biasa (opak, akses field lewat jalur dinamis seperti biasa), fungsinya tetap benar tapi tidak dapat percepatan ini.

---

## Kompilasi AOT (Executable Mandiri)

```bash
isoteri bangun program.iso -o nama_keluaran
./nama_keluaran           catatan: jalan langsung, gak butuh 'isoteri' atau berkas .iso lagi
```

Subcommand `bangun` mengompilasi program `.iso` (beserta semua yang di-`muat`-nya) jadi satu **executable native mandiri** — bisa didistribusikan dan dijalankan di komputer lain tanpa perlu instalasi apa pun (tidak butuh `isoteri`, tidak butuh Rust, tidak butuh berkas `.iso` sumbernya).

### Cara kerja

1. Semua berkas yang terhubung lewat `muat` (mulai dari `program.iso`, termasuk `muat X sebagai alias`) diproses lewat jalur AST yang sama dipakai jalur eksekusi biasa (`program_dari_berkas`) — bukan lagi tempel-teks murni, jadi alias modul berfungsi penuh di sini juga.
2. Hasilnya dicetak ulang jadi satu teks sumber gabungan (`cetak_program_ke_sumber`) — baris `muat "..."` sudah hilang karena isinya sudah ditempel/di-mangle langsung sesuai aturan alias.
3. Sumber gabungan itu divalidasi dulu (lexer, parser, resolver) — kalau ada error bahasa Isoteri, langsung ketahuan di sini (nyaris instan), **sebelum** buang waktu kompilasi Rust yang jauh lebih lambat.
4. Sumber gabungan ditempel sebagai string literal ke sebuah crate Rust kecil yang cuma memanggil `isoteri::jalankan_sumber(...)`, lalu di-`cargo build --release`.

### Opsi

| Flag | Keterangan |
|---|---|
| `-o <nama>` / `--keluaran <nama>` | Nama file executable hasil. Default: nama file masukan tanpa ekstensi `.iso`. |

### Performa build

Build **pertama kali** butuh beberapa menit (kompilasi seluruh dependency — Cranelift, `ureq`, dst. — dari nol). Build-build **berikutnya** jauh lebih cepat (hitungan detik) karena dependency yang sudah dikompilasi disimpan di cache kerja yang persisten (`$TMPDIR/isoteri-bangun-cache`) dan dipakai ulang oleh Cargo secara incremental — bahkan untuk program `.iso` yang berbeda-beda, selama dependency-nya (yakni Isoteri sendiri) tidak berubah.

### Batasan

- Butuh Rust & Cargo terpasang di mesin **yang dipakai untuk bangun** (bukan di mesin yang nanti menjalankan hasil executable-nya — hasil buildnya sudah mandiri).
- Hasil executable spesifik untuk platform (OS + arsitektur CPU) tempat ia dibangun — belum ada cross-compilation.
