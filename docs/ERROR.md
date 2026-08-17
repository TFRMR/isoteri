# Panduan Error & Debugging

Semua pesan error di Isoteri diawali salah satu dari lima label. Label ini langsung memberi tahu **di tahap mana** masalahnya terjadi — penting untuk tahu, karena menentukan cara kamu memperbaikinya.

| Label | Tahap | Bisa ditangkap `coba/tangkap`? |
|---|---|---|
| `Kesalahan Lexer` | Membaca karakter mentah jadi token | Tidak |
| `Kesalahan Parser` | Menyusun token jadi struktur program | Tidak |
| `Kesalahan Muat` | Menggabungkan file lewat `muat` | Tidak |
| `Kesalahan Kompilasi` | Memvalidasi & menyiapkan program sebelum jalan | Tidak |
| `Kesalahan Runtime` | Program sudah jalan, error terjadi di tengah eksekusi | **Ya** |

Empat yang pertama terjadi **sebelum** program sempat jalan sama sekali — kalau salah satu ini muncul, program berhenti total dan tidak mencetak apa pun dari `tampilkan`, meski `tampilkan`-nya ada di baris sebelum letak errornya. Cuma `Kesalahan Runtime` yang bisa ditangkap dengan `coba { ... } tangkap pesan { ... }`.

---

## `Kesalahan Lexer`

Terjadi kalau ada karakter yang tidak dikenal bahasa ini sama sekali (mis. simbol aneh, kutip yang tidak ditutup).

Contoh pesan:
```
Kesalahan Lexer: Karakter '!' tidak dikenal pada baris 3
```

**Cara perbaiki**: cek baris yang disebutkan, cari karakter yang salah ketik atau tanda kutip yang lupa ditutup.

---

## `Kesalahan Parser`

Terjadi kalau token-nya valid satu-satu, tapi susunannya tidak membentuk struktur program yang sah (kurung tidak lengkap, kata kunci di tempat yang salah, dst.).

Contoh pesan asli:
```
Kesalahan Parser: Baris 1: Diharapkan KurungBuka, tapi ditemukan Identifikator("x")
```

Ini muncul dari kode `kalau x { ... }` — lupa mengurung kondisinya. Yang benar: `kalau (x) { ... }`.

**Cara baca pesan seperti ini**: "Diharapkan `TOKEN_A`, tapi ditemukan `TOKEN_B`" artinya di titik itu Isoteri mengharapkan token jenis `TOKEN_A` (nama token internal, bukan istilah dalam bahasa Isoteri — `KurungBuka` = `(`, `KurawalBuka` = `{`, `TitikDua` = `:`, dst.), tapi malah menemukan `TOKEN_B`. Biasanya artinya ada yang lupa ditulis (kurung, koma, titik dua) tepat sebelum posisi error.

**Penyebab umum:**
- Lupa mengurung kondisi `kalau`/`ulang` dengan `(...)`.
- Lupa koma antar parameter fungsi atau antar item `Daftar`/`Peta`.
- Lupa `titik dua` (`:`) antara nama field dan tipenya di `bentuk`, atau antara nama field dan nilainya di literal `bentuk`/`Peta`.

---

## `Kesalahan Muat`

Khusus untuk masalah terkait statement `muat "path.iso"`.

Contoh pesan asli:
```
Kesalahan Muat: Baris 1: Tidak bisa memuat "tidak_ada.iso": No such file or directory (os error 2)
```

```
Kesalahan Muat: Nama "proses" dideklarasikan di dua modul berbeda: fungsi "proses" di [modul_a.iso],
dan fungsi "proses" di [modul_b.iso]. Ganti salah satu nama supaya gak tabrakan.
```

**Penyebab umum:**
- Path di `muat "..."` salah ketik, atau relatif ke direktori yang salah (ingat: path dihitung dari lokasi file yang menulis `muat`-nya, bukan selalu dari file yang pertama dijalankan — lihat [REFERENSI.md](REFERENSI.md#modul-muat)).
- Dua modul berbeda kebetulan mendefinisikan fungsi/`bentuk`/variabel global dengan nama sama — ganti salah satu namanya.

---

## `Kesalahan Kompilasi`

Kategori paling besar — mencakup semua validasi sebelum program dijalankan: variabel/fungsi tidak dikenal, tipe tidak cocok, field `bentuk` bermasalah, deklarasi ganda.

### Variabel tidak ditemukan
```
Kesalahan Kompilasi: Variabel "x" tidak ditemukan. Apakah sudah dideklarasikan dengan 'ingat'?
Kesalahan Kompilasi: Variabel "y" belum dideklarasikan dengan 'ingat'.
```
Muncul kalau kamu memakai nama variabel yang belum pernah dideklarasikan `ingat` di scope itu, atau salah ketik nama variabel. Ingat: variabel global harus dideklarasikan **sebelum** dipakai secara tekstual (tidak seperti fungsi/`bentuk`).

### Tipe tidak cocok
```
Kesalahan Kompilasi: Kesalahan Tipe: variabel "a" bertipe Angka, tapi diberi nilai bertipe Teks.
```
Muncul kalau kamu memberi anotasi tipe eksplisit (`ingat a: Angka = ...`) tapi nilainya tidak cocok. Perbaiki dengan mengubah nilainya, atau melepas anotasi tipenya kalau memang sengaja fleksibel.

### Field `bentuk` bermasalah
```
Kesalahan Kompilasi: Bentuk "P" butuh field "b" yang belum diisi.
Kesalahan Kompilasi: Bentuk "P" tidak punya field "c".
Kesalahan Kompilasi: Bentuk "P": field "a" diisi lebih dari sekali.
```
Ketiganya muncul saat membuat instans `bentuk` (`P { ... }`) — field yang kurang, field yang tidak ada di definisi, atau field yang ditulis dua kali. Cek definisi `bentuk`-nya dan cocokkan persis field mana yang dibutuhkan.

### Deklarasi ganda
```
Kesalahan Kompilasi: Fungsi "f" sudah dideklarasikan sebelumnya.
Kesalahan Kompilasi: Fungsi "f": parameter "a" dipakai lebih dari sekali.
Kesalahan Kompilasi: Bentuk "P" sudah dideklarasikan sebelumnya.
```
Muncul kalau ada dua `fungsi` bernama sama di file yang sama, dua parameter bernama sama di satu fungsi, atau dua `bentuk` bernama sama.

---

## `Kesalahan Runtime`

Terjadi **saat program sudah jalan**. Ini satu-satunya kategori yang bisa ditangkap dengan `coba/tangkap` — lihat [TUTORIAL.md](TUTORIAL.md#9-menangani-error). Selalu diawali `Baris N:` yang menunjuk baris tempat error terjadi (kecuali errornya berasal dari file yang di-`muat`, nomor barisnya tetap relatif ke file itu tapi nama filenya tidak disebutkan — lihat [KETERBATASAN.md](KETERBATASAN.md)).

### Pembagian dengan nol
```
Kesalahan Runtime: Baris 1: Tidak bisa membagi dengan nol.
```

### Indeks di luar jangkauan
```
Kesalahan Runtime: Baris 2: Indeks 5 di luar jangkauan (panjang daftar: 2)
```
Ingat, indeks `Daftar` mulai dari `0` — daftar berisi 2 item punya indeks valid `0` dan `1` saja.

### Fungsi/field tidak ditemukan saat dipanggil
```
Kesalahan Runtime: Baris 1: Fungsi "fungsi_gaib" tidak ditemukan.
Kesalahan Runtime: Baris 2: Akses field ".a" hanya berlaku untuk instans 'bentuk', ditemukan 5
```
Yang pertama: salah ketik nama fungsi bawaan/buatan sendiri. Yang kedua: mencoba akses `.field` pada nilai yang bukan instans `bentuk` (mis. `Angka`) — field access dengan titik (`.`) cuma berlaku untuk `bentuk`, pakai `[...]` untuk `Daftar`/`Peta`.

### Memanggil nilai yang bukan fungsi
```
Kesalahan Runtime: Baris 2: Nilai ini bukan fungsi, gak bisa dipanggil: 5
```
Muncul kalau kamu menulis `f(...)` di mana `f` ternyata berisi nilai biasa (Angka, Teks, dst.), bukan closure.

### Argumen closure tidak cocok jumlahnya
```
Kesalahan Runtime: Baris 2: Fungsi ini butuh 2 argumen, tapi diberi 1.
```

---

## Strategi Debugging Umum

1. **Baca label errornya dulu.** Kalau `Kesalahan Kompilasi`/`Parser`/`Lexer`/`Muat`, masalahnya ada di STRUKTUR/PENULISAN kode, bukan logika jalannya — program belum sempat dijalankan. Kalau `Kesalahan Runtime`, kodenya sudah valid secara struktur, masalahnya ada di data/kondisi saat program jalan.
2. **Cari nomor barisnya**, tapi ingat kalau ada `muat`, nomor baris itu relatif ke file tempat statement bermasalah itu ditulis, bukan selalu file yang kamu jalankan.
3. **Untuk error runtime yang mau kamu antisipasi** (misalnya potensi pembagian nol dari input pengguna), bungkus dengan `coba/tangkap` supaya program tidak berhenti total.
4. **Kalau bingung nilai suatu variabel di tengah program**, tambahkan `tampilkan` sementara untuk mengintip nilainya — belum ada debugger interaktif di Isoteri (lihat [KETERBATASAN.md](KETERBATASAN.md)).
5. **Kalau errornya soal tipe/field yang membingungkan padahal kelihatannya sudah benar**, cek ulang definisi `bentuk`-nya persis (nama field, urutan tidak masalah tapi ejaan harus persis sama, termasuk huruf besar-kecil).
