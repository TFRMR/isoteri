# Isoteri Web Runtime (Fase 3 -- Browser Native)

Menjalankan program Isoteri di browser **tanpa server dan tanpa WASM asli**,
dengan mengekspor bytecode terkompilasi ke JSON lalu menjalankannya lewat
interpreter JavaScript (`isoteri-vm.js`) yang semantiknya ditulis ulang
persis mengikuti VM Rust (`src/lib.rs`).

## Kenapa bukan WASM langsung?

Blueprint asli menargetkan `Isoteri -> IR -> WASM -> Browser`. Itu tetap
arah jangka panjang yang benar (lebih cepat dari interpreter JS). Tapi
mengompilasi ke WASM butuh target `wasm32-unknown-unknown`, yang di banyak
environment (termasuk environment pengembangan proyek ini) hanya tersedia
lewat `rustup target add`, dan `rustup` sendiri sering tidak terpasang atau
tidak bisa mengunduh komponennya (butuh akses ke `static.rust-lang.org`
yang kerap diblokir jaringan sandbox/CI).

Jalan pragmatis: bytecode Isoteri itu sendiri sudah representasi flat &
portable (array instruksi berisi angka/teks/enum sederhana -- lihat
`enum Instr` di `src/lib.rs`). Jadi cukup di-dump ke JSON, lalu dijalankan
oleh VM kecil di JavaScript. Hasilnya: janji "Browser Native" terpenuhi
*hari ini*, tanpa menunggu toolchain wasm32. Begitu wasm32 tersedia,
jalur "compile ke WASM asli" tetap layak dikerjakan sebagai peningkatan
performa -- dua-duanya bisa hidup berdampingan.

## Cara pakai

```bash
# 1. Kompilasi program .iso jadi bundel bytecode JSON
isoteri ekspor-web program.iso -o program.isoweb.json

# 2a. Jalankan di Node.js (buat testing/CI)
node runtime/web/jalankan-node.js program.isoweb.json

# 2b. Atau jalankan di browser: buka runtime/web/demo.html,
#     atau pakai isoteri-vm.js langsung:
```

```html
<script src="isoteri-vm.js"></script>
<script>
  fetch("program.isoweb.json")
    .then((r) => r.json())
    .then((bundle) => new IsoteriVM(bundle, {
      tampilkan: (baris) => console.log(baris), // atau tulis ke DOM
    }).jalankan());
</script>
```

## Yang didukung (identik dengan native, sudah diuji lewat perbandingan output)

Semua fitur bahasa inti: variabel, `kalau/lainnya`, `ulang`, `ulang setiap`,
fungsi & rekursi, closure (termasuk closure bersarang & menangkap banyak
variabel), `bentuk` (struct) + akses/ubah field, `coba/tangkap` (termasuk
bersarang), `petakan`/`saring`/`urutkan` dengan callback nama fungsi,
serta hampir semua fungsi bawaan: `panjang`, `gabung`, `ambil`, `jumlah`,
`rata_rata`, `kunci_peta`, konversi tipe, fungsi matematika (`akar`,
`pangkat`, `bulat*`, `mutlak`, `min`/`maks`, `acak`), fungsi teks
(`potong`, `ganti`, `huruf_besar/kecil`, `pangkas`, `pisah`, `satukan`,
`mengandung`, `diawali`, `diakhiri`), serta `urai_json`/`teks_json`.

Fungsi hasil JIT (Cranelift) tetap jalan benar di web -- diekspor sebagai
bytecode fallback biasa (bukan kode mesin native, yang memang tak mungkin
dikirim ke browser), jadi hasilnya identik walau tanpa percepatan JIT.

Diverifikasi otomatis: 13 dari 16 program contoh di root proyek
menghasilkan output **identik byte-per-byte** antara `isoteri program.iso`
(native) dan `node jalankan-node.js program.isoweb.json` (web).

## Yang BELUM didukung (disengaja)

- **`ulang selaras`** (`JalankanSelaras`) -- instruksi ini menyimpan AST
  mentah (bukan bytecode flat), jadi tidak ikut diekspor. `isoteri ekspor-web`
  akan menolak dengan pesan jelas kalau programnya memakai ini. Solusi
  sementara: pakai `ulang setiap` biasa untuk kode yang perlu jalan di
  browser juga.
- **`unduh()`, `baca_berkas()`, `tulis_berkas()`** -- I/O jaringan/sistem
  file sinkron, sengaja tidak diimplementasi di sandbox browser (beda model
  keamanan). Baris ini akan melempar error yang jelas kalau dipanggil;
  bungkus dengan `coba/tangkap` kalau perlu tetap jalan sebagian di web.
  Ke depan: bisa dipetakan ke `fetch()` (async) dan `window.storage`/
  `IndexedDB`, tapi itu perlu VM ini jadi async -- belum dikerjakan di v1.

## Milestone B -- DOM/Event/Storage/Fetch

Fungsi bawaan baru (di `isoteri-vm.js`, TIDAK ada perubahan di sisi Rust
sama sekali -- konsisten dengan Hukum 3 di `docs/FILOSOFI.md`: DOM adalah
lapisan platform, bukan bagian core language):

**DOM** -- elemen direpresentasikan sebagai `Instans("ElemenDOM", ...)`, jadi
tetap nilai Isoteri biasa (bisa disimpan di variabel, diteruskan sebagai
argumen, dst):
```isoteri
ingat judul = dom_pilih("#judul")           catatan: querySelector, Kosong kalau tidak ada
ingat semua = dom_pilih_semua(".item")      catatan: querySelectorAll -> Daftar

dom_teks(judul)                              catatan: baca textContent
dom_atur_teks(judul, "Halo Isoteri")         catatan: set textContent
dom_html(judul)                              dom_atur_html(judul, "<b>halo</b>")
dom_atribut(judul, "data-id")                dom_atur_atribut(judul, "data-id", "5")
dom_tambah_kelas(judul, "aktif")             dom_hapus_kelas(judul, "aktif")
dom_punya_kelas(judul, "aktif")              catatan: -> Bool

ingat baru = dom_buat("span")                catatan: createElement
dom_tambah_anak(judul, baru)                 catatan: appendChild
dom_hapus(baru)                              catatan: .remove()
```

**Event** -- konvensi SAMA seperti `petakan`/`saring`/`urutkan`: nama
fungsi penangan lewat Teks, bukan referensi first-class (bahasa belum
punya sintaks buat itu -- lihat catatan di `IsoteriVM.panggilDom`):
```isoteri
fungsi ketika_diklik() { tampilkan "Diklik!" }
dom_ketika(tombol, "klik", "ketika_diklik")
```

**Storage** (localStorage):
```isoteri
simpan_lokal("kunci", "nilai")
ambil_lokal("kunci")     catatan: -> Teks atau Kosong kalau belum ada
hapus_lokal("kunci")
```

**Fetch** (async, beda dari `unduh()` yang sengaja TIDAK didukung karena
sinkron -- lihat bagian "Yang BELUM didukung" di atas):
```isoteri
fungsi saat_sukses(isi) { tampilkan isi }
fungsi saat_gagal(pesan) { tampilkan "Gagal: " + pesan }
unduh_async("https://api.contoh.com/data", "saat_sukses", "saat_gagal")
```

Lihat `runtime/web/contoh_dom.iso` buat contoh lengkap semua fungsi di atas,
dan `docs/IR.md`/`docs/FILOSOFI.md` buat status Milestone B secara umum.

## Canvas 2D & WebSocket (lanjutan Milestone B)

**Canvas** -- pilih elemen `<canvas>` seperti biasa lewat `dom_pilih`, lalu
ambil konteks gambarnya:
```isoteri
ingat kanvas = dom_pilih("#papan")
ingat ctx = dom_konteks_2d(kanvas)          catatan: -> Instans "Konteks2D"

kanvas_isi_gaya(ctx, "merah")               catatan: fillStyle
kanvas_garis_gaya(ctx, "biru")              catatan: strokeStyle
kanvas_lebar_garis(ctx, 3)                  catatan: lineWidth
kanvas_font(ctx, "16px sans-serif")

kanvas_isi_persegi(ctx, 10, 10, 100, 50)    catatan: fillRect(x,y,lebar,tinggi)
kanvas_garis_persegi(ctx, 10, 10, 100, 50)  catatan: strokeRect(...)
kanvas_bersihkan(ctx, 0, 0, 200, 200)       catatan: clearRect(...)
kanvas_isi_teks(ctx, "Halo", 5, 90)         catatan: fillText(teks,x,y)

kanvas_mulai_jalur(ctx)                     catatan: beginPath
kanvas_pindah_ke(ctx, 0, 0)                 catatan: moveTo
kanvas_garis_ke(ctx, 50, 50)                catatan: lineTo
kanvas_lingkaran(ctx, 50, 50, 20, 0, 6.28)  catatan: arc(x,y,radius,sudut_mulai,sudut_akhir)
kanvas_garis(ctx)                            catatan: stroke()
kanvas_isi(ctx)                              catatan: fill()
```

**WebSocket** -- direpresentasikan sebagai `Instans "WebSocket"`, event lewat
konvensi nama-fungsi-string yang sama:
```isoteri
fungsi saat_pesan(isi) { tampilkan "Diterima: " + isi }
fungsi saat_buka() { tampilkan "Tersambung" }

ingat soket = ws_buka("wss://contoh.com/socket")
ws_ketika_buka(soket, "saat_buka")
ws_ketika_pesan(soket, "saat_pesan")     catatan: fungsi penangan terima 1 argumen (Teks isi pesan)
ws_ketika_tutup(soket, "saat_buka")      catatan: bisa pakai fungsi yang sama buat event lain
ws_ketika_error(soket, "saat_buka")

ws_kirim(soket, "halo server")
ws_status(soket)                          catatan: -> "MENYAMBUNG"/"TERBUKA"/"MENUTUP"/"TERTUTUP"
ws_tutup(soket)
```

Lihat `runtime/web/contoh_kanvas_ws.iso` buat contoh lengkap.

## Arsitektur singkat

```
program.iso --[isoteri ekspor-web]--> program.isoweb.json
                                            |
                                            v
                                   isoteri-vm.js (browser/Node)
                                            |
                                            v
                                   tampilkan() -> DOM/console
```

Lihat komentar di `src/lib.rs` (fungsi `ekspor_json_dari_sumber`,
`instr_ke_json`, `value_ke_json`) untuk skema JSON persisnya, dan
`isoteri-vm.js` untuk implementasi VM-nya (`class IsoteriVM`).
