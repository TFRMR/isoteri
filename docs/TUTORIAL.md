# Tutorial Isoteri: Dari Nol

Tutorial ini mengajak kamu nulis program Isoteri langkah demi langkah, dari yang paling dasar sampai fitur-fitur yang lebih canggih. Setiap potongan kode di sini sudah diuji dan dijamin jalan persis seperti yang ditulis — kamu bisa salin-tempel langsung ke file `.iso` dan jalanin dengan `isoteri nama_file.iso`.

Kalau ada istilah yang belum jelas, cek [REFERENSI.md](REFERENSI.md) untuk detail lengkapnya. Kalau nemu error yang membingungkan, cek [ERROR.md](ERROR.md).

---

## 1. Program Pertama

Buat file `halo.iso`:

```
tampilkan "Halo, Isoteri!"
```

Jalankan:
```bash
isoteri halo.iso
```

Output:
```
Halo, Isoteri!
```

`tampilkan` adalah cara Isoteri mencetak sesuatu ke layar — semacam `print()` di bahasa lain.

---

## 2. Variabel & Tipe Data

```
ingat nama = "Budi"
ingat umur: Angka = 28
ingat tinggi: Desimal = 165.5
ingat petani = benar

tampilkan nama
tampilkan "Umur: " + umur
```

- `ingat` mendeklarasikan variabel baru. `simpan` adalah kata lain yang sama persis artinya, pakai mana saja sesuai selera.
- Anotasi tipe (`: Angka`, `: Desimal`) itu **opsional** — Isoteri bisa menyimpulkan tipe dari nilainya. Tapi kalau kamu tulis anotasinya, Isoteri akan **memvalidasinya saat kompilasi** — kalau kamu keliru kasih nilai bertipe salah, program akan gagal dibangun dengan pesan error yang jelas, bahkan sebelum sempat dijalankan.
- Untuk mengubah nilai variabel yang sudah ada, jangan pakai `ingat` lagi — cukup `nama_variabel = nilai_baru`.

Empat tipe dasar yang paling sering dipakai: `Teks` (string), `Angka` (bilangan bulat), `Desimal` (bilangan pecahan), `Bool` (`benar`/`salah`).

---

## 3. Percabangan

```
kalau (umur >= 17) {
    tampilkan nama + " sudah dewasa"
} lainnya {
    tampilkan nama + " belum dewasa"
}
```

- `kalau` sama artinya dengan "if" di bahasa lain. `jika` adalah sinonimnya, boleh dipakai bergantian.
- Kondisinya **wajib** dikurung `(...)`.
- Blok `lainnya` (else) sifatnya opsional.

---

## 4. Perulangan

### Loop dengan kondisi (`ulang`)

```
ingat i = 0
ulang (i < 3) {
    tampilkan "Perulangan ke-" + i
    i = i + 1
}
```

### Loop lewat daftar (`ulang setiap`)

```
ingat hasil_panen = [120, 95, 150, 88]
ulang setiap h dari hasil_panen {
    tampilkan "Panen: " + h + " kg"
}
```

`[120, 95, 150, 88]` adalah literal `Daftar` (list) — kumpulan nilai berurutan, bisa diakses lewat indeks (`hasil_panen[0]`) atau dijelajahi satu-satu lewat `ulang setiap`.

---

## 5. Fungsi

```
fungsi hitung_total(harga_per_kg: Angka, berat_kg: Angka) {
    kembalikan harga_per_kg * berat_kg
}

tampilkan "Total: " + hitung_total(5000, 25)
```

- `kembalikan` mengirim nilai balik dari fungsi (seperti `return`).
- Anotasi tipe parameter opsional, sama seperti variabel — tapi kalau semua parameter & variabel lokal fungsi kamu bertipe seragam (semua `Angka` atau semua `Desimal`), Isoteri otomatis mengompilasi fungsi itu jadi **kode mesin asli** (lewat JIT) untuk performa maksimal, tanpa perlu kamu lakukan apa pun secara manual. Detail lengkap ada di [REFERENSI.md](REFERENSI.md#kompilasi-jit).
- Fungsi boleh dipanggil sebelum baris deklarasinya (beda dari variabel, yang harus dideklarasikan dulu baru bisa dipakai).

---

## 6. Bentuk (Struct) — Bikin Tipe Data Sendiri

Kalau kamu perlu mengelompokkan beberapa nilai jadi satu "benda" (misalnya data seorang petani: nama, luas lahan, hasil panen), pakai `bentuk`:

```
bentuk Petani {
    nama: Teks,
    lahan_hektar: Angka,
    hasil_panen: Desimal
}

ingat p1 = Petani { nama: "Siti", lahan_hektar: 2, hasil_panen: 340.5 }
tampilkan p1.nama + " punya " + p1.lahan_hektar + " hektar"

p1.hasil_panen = 360.0
tampilkan p1
```

Beberapa hal penting soal `bentuk`:
- Semua field **wajib diisi** waktu bikin instans — kalau ada yang kelupaan, atau ada field asing yang gak ada di definisinya, atau tipenya salah, Isoteri akan menolaknya **saat kompilasi** (sebelum program sempat jalan), dengan pesan yang menyebutkan persis field mana yang bermasalah.
- Urutan field di literal (`Petani { nama: ..., lahan_hektar: ..., hasil_panen: ... }`) bebas, tidak harus sama urutan dengan definisinya.
- Kamu bisa mengubah field-nya (`p1.hasil_panen = 360.0`), termasuk field yang bersarang di dalam field lain (`objek.alamat.desa = "..."`, sedalam apapun).

---

## 7. Menggabungkan Semuanya: Daftar Berisi `Bentuk`

Ini yang bikin `bentuk` benar-benar berguna — kombinasikan dengan `Daftar` dan fungsi list bawaan untuk mengolah kumpulan data:

```
fungsi ambil_hasil(p) { kembalikan p.hasil_panen }

ingat semua_petani = [
    Petani { nama: "Budi", lahan_hektar: 1, hasil_panen: 200.0 },
    Petani { nama: "Siti", lahan_hektar: 2, hasil_panen: 360.0 },
    Petani { nama: "Anto", lahan_hektar: 1, hasil_panen: 150.0 }
]

ingat terurut = urutkan(semua_petani, "ambil_hasil")
ulang setiap p dari terurut {
    tampilkan p.nama + ": " + p.hasil_panen + " kg"
}
```

Output:
```
Anto: 150.0 kg
Budi: 200.0 kg
Siti: 360.0 kg
```

`urutkan(daftar, "nama_fungsi")` mengurutkan daftar berdasarkan nilai yang dikembalikan `nama_fungsi` untuk tiap item — di sini, berdasarkan `hasil_panen`. Perhatikan nama fungsinya ditulis sebagai Teks (`"ambil_hasil"`), bukan `ambil_hasil` telanjang — ini karena `urutkan`/`petakan`/`saring` belum mendukung closure langsung sebagai argumen (lihat [KETERBATASAN.md](KETERBATASAN.md)).

Fungsi list bawaan lain yang berguna: `petakan(daftar, "fn")` (map/transformasi tiap item), `saring(daftar, "fn")` (filter, `fn` harus kembalikan `Bool`). Daftar lengkap fungsi bawaan ada di [REFERENSI.md](REFERENSI.md#fungsi-bawaan-standard-library).

---

## 8. Closure — Fungsi yang "Mengingat"

Closure adalah fungsi tanpa nama yang bisa "mengingat" nilai dari tempat ia dibuat:

```
fungsi buat_pengali(faktor) {
    kembalikan fungsi(x) { kembalikan x * faktor }
}

ingat kali_dua = buat_pengali(2)
ingat kali_sepuluh = buat_pengali(10)

tampilkan "kali_dua(5) = " + kali_dua(5)         catatan: 10
tampilkan "kali_sepuluh(5) = " + kali_sepuluh(5)  catatan: 50
```

`buat_pengali(2)` mengembalikan sebuah closure yang "mengingat" bahwa `faktor` = 2. Setiap kali `buat_pengali` dipanggil dengan angka berbeda, closure yang dihasilkan mengingat angka yang berbeda pula — cocok untuk membuat fungsi-fungsi khusus secara dinamis (misalnya kalkulator pajak/diskon dengan tarif berbeda-beda).

Satu hal penting: yang "diingat" closure itu **nilai pada saat closure dibuat**, bukan referensi hidup. Kalau `faktor` di atas berubah setelah `kali_dua` dibuat, `kali_dua` tetap pakai nilai `faktor` yang lama. Detail lengkap soal closure ada di [REFERENSI.md](REFERENSI.md#closure-fungsi-anonim) dan batasannya di [KETERBATASAN.md](KETERBATASAN.md).

---

## 9. Menangani Error

```
coba {
    ingat x = 10 / 0
} tangkap pesan {
    tampilkan "Terjadi error: " + pesan
}
```

`coba { ... } tangkap pesan { ... }` menjalankan blok pertama, dan kalau terjadi error saat program **berjalan** (pembagian dengan nol, indeks di luar jangkauan, dst.), eksekusi langsung lompat ke blok `tangkap` dengan pesan error tersimpan di variabel `pesan`.

Penting: ini cuma menangkap error yang terjadi **saat program jalan**. Error yang terdeteksi **saat kompilasi** (misalnya field `bentuk` yang kurang, atau tipe yang salah) tidak bisa ditangkap `coba/tangkap`, karena program belum sempat mulai jalan sama sekali kalau errornya jenis itu.

---

## 10. Memecah Program Jadi Beberapa File

Begitu program kamu makin besar, pecah jadi beberapa file `.iso` dan gabungkan lewat `muat`:

**`bentuk_petani.iso`**
```
bentuk Petani {
    nama: Teks,
    lahan_hektar: Angka,
    hasil_panen: Desimal
}
```

**`main.iso`**
```
muat "bentuk_petani.iso"

ingat p1 = Petani { nama: "Budi", lahan_hektar: 1, hasil_panen: 200.0 }
tampilkan p1
```

`muat "path.iso"` menempelkan isi file lain ke program kamu (path dihitung relatif dari file yang menulis `muat`-nya). Kalau dua modul kebetulan punya fungsi/bentuk/variabel dengan nama sama, Isoteri akan memberi tahu lewat error yang jelas saat kompilasi, bukan diam-diam membiarkan salah satunya ketiban. Detail lengkap ada di [REFERENSI.md](REFERENSI.md#modul-muat).

---

## Langkah Selanjutnya

- Baca [REFERENSI.md](REFERENSI.md) untuk daftar lengkap sintaks, operator, dan semua fungsi bawaan.
- Baca [KETERBATASAN.md](KETERBATASAN.md) supaya tahu batasan yang sudah diketahui sebelum menganggapnya bug.
- Kalau kena error yang membingungkan, cek [ERROR.md](ERROR.md).
