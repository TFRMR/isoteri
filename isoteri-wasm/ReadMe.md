# Isoteri Web Runtime (Fase 3 -- Browser Native)

Menjalankan program Isoteri di browser lewat dua jalur yang hidup
berdampingan:

1. **Jalur JSON+VM-JS** (jalur pragmatis awal, masih dipakai/didukung):
   mengekspor bytecode terkompilasi ke JSON lalu menjalankannya lewat
   interpreter JavaScript (`isoteri-vm.js`) yang semantiknya ditulis ulang
   persis mengikuti VM Rust (`src/lib.rs`).
2. **Jalur WASM asli** (`isoteri-wasm/`, sekarang sudah dibangun &
   divalidasi -- lihat `demo_wasm.html`): source `.iso` dikompilasi
   langsung di browser lewat compiler Rust yang sama, di-compile ke
   `wasm32-unknown-unknown`. Hasilnya tetap dijalankan oleh `isoteri-vm.js`
   yang sama persis dipakai jalur 1 -- yang berbeda cuma CARA bundel
   JSON-nya dihasilkan (di browser vs lewat CLI), bukan VM eksekusinya.

## Kenapa dua jalur, bukan cuma satu?

Blueprint asli menargetkan `Isoteri -> IR -> WASM -> Browser`. Mengompilasi
ke WASM butuh target `wasm32-unknown-unknown`, yang di banyak environment
(termasuk environment pengembangan sandbox proyek ini) hanya tersedia lewat
`rustup target add`, dan `rustup` sendiri sering tidak terpasang atau tidak
bisa mengunduh komponennya (butuh akses ke `static.rust-lang.org` yang
kerap diblokir jaringan sandbox/CI).

Jalan pragmatis (jalur 1) dibuat lebih dulu: bytecode Isoteri itu sendiri
sudah representasi flat & portable (array instruksi berisi angka/teks/enum
sederhana -- lihat `enum Instr` di `src/lib.rs`). Jadi cukup di-dump ke
JSON, lalu dijalankan oleh VM kecil di JavaScript. Ini memenuhi janji
"Browser Native" tanpa menunggu toolchain wasm32, dan **tetap dipertahankan**
sebagai fallback untuk environment yang tidak bisa build wasm32 (mis. CI
tanpa akses `static.rust-lang.org`).

Begitu toolchain wasm32 tersedia (di mesin lokal dengan akses internet
penuh), jalur 2 dikerjakan dan **sudah berhasil**: source `.iso` mentah
sekarang bisa dikompilasi langsung di browser tanpa langkah CLI/ekspor
bundel terpisah sama sekali. Lihat `../../isoteri-wasm/README.md` untuk
cara build ulang `pkg/`, dan `demo_wasm.html` di folder ini untuk demo
jalur 2. Belum divalidasi lewat jalur 2 untuk fitur bahasa yang lebih
kompleks (struct, closure, loop, DOM binding penuh) -- baru fungsi, string,
dan aritmatika dasar.

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

**Event** -- bisa closure (dengan capture) ATAU nama fungsi lewat Teks, dan
sekarang dapat SATU argumen opsional berisi data event (backward-compatible
penuh dengan handler 0-parameter lama):
```isoteri
dom_ketika(tombol, "klik", fungsi() { tampilkan "Diklik!" })      catatan: 0 parameter, gaya lama
dom_ketika(input, "input", fungsi(e) { tampilkan e.nilai })       catatan: 1 parameter -- baca data event
dom_ketika(tombol, "klik", "nama_fungsi")                           catatan: Teks, gaya lama, tetap jalan
```
`e` adalah instans `Event` dengan field `tipe`, `nilai` (isi `.value` target
kalau ada), `tombol` (tombol keyboard kalau event keyboard), `target`
(`ElemenDOM`). Form input: `dom_nilai`/`dom_atur_nilai`/`dom_dicentang`/
`dom_atur_dicentang`/`dom_fokus`.

**Storage** (localStorage):
```isoteri
simpan_lokal("kunci", "nilai")
ambil_lokal("kunci")     catatan: -> Teks atau Kosong kalau belum ada
hapus_lokal("kunci")
```

**Timer**:
```isoteri
tunda(1000, fungsi() { tampilkan "sedetik kemudian" })          catatan: setTimeout
ingat id = interval_mulai(500, fungsi() { tampilkan "tik" })     catatan: setInterval
interval_hentikan(id)
```

**Fetch** (async, beda dari `unduh()` yang sengaja TIDAK didukung karena
sinkron -- lihat bagian "Yang BELUM didukung" di atas):
```isoteri
unduh_async("https://api.contoh.com/data", fungsi(isi) { tampilkan isi })   catatan: GET-teks, sederhana
unduh_lanjut_async("https://api.contoh.com/data",
    {"metode": "POST", "body": teks_json(data), "header": {"Content-Type": "application/json"}},
    fungsi(r) { tampilkan r.status; tampilkan urai_json(r.teks) },
    fungsi(pesan) { tampilkan "gagal: " + pesan })
```

Lihat `runtime/web/contoh_dom.iso` buat contoh lengkap semua fungsi di atas,
dan `runtime/web/contoh_event_form_timer/` (buka `index.html` lewat local
server) buat demo interaktif event closure + data event + form input + timer
sekaligus dalam satu halaman.
dan `docs/IR.md`/`docs/FILOSOFI.md` buat status Milestone B secara umum.

## Router, State Management, Component System (di atas Milestone B)

Fondasi buat aplikasi web kompleks (dashboard, CRUD, e-commerce
skala-menengah), semua murni JavaScript di `isoteri-vm.js`, nol perubahan
ke compiler/VM Rust:

```isoteri
catatan: Router -- hash-based (#/path), zero-config di hosting statis apa pun
rute_daftar([
    {"pola": "/", "tampilkan": "render_beranda"},
    {"pola": "/produk/:id", "tampilkan": "render_produk"},
    {"pola": "*", "tampilkan": "render_404"}
])
rute_mulai()
rute_navigasi("/produk/7")
rute_sekarang()              catatan: {path, params}

catatan: State Management -- pub/sub sederhana
ingat toko = state_buat(0)
state_langgan(toko, fungsi(n) { dom_atur_teks(el, "" + n) })
state_atur(toko, 5)
state_ubah(toko, fungsi(lama) { kembalikan lama + 1 })

catatan: Component System -- render-ulang-penuh + event delegation lewat data-aksi
ingat komp = komponen_buat({
    "state_awal": 0,
    "render": fungsi(props, state) {
        kembalikan "<button data-aksi='tambah'>" + state + "</button>"
    },
    "aksi": { "tambah": fungsi(props, state, e) { kembalikan state + 1 } },
    "dipasang": fungsi(props, state) { tampilkan "siap" }
})
ingat inst = komponen_pasang(komp, dom_pilih("#app"))

catatan: Nested/composed components -- komponen_anak() DI DALAM render() induk, kunci stabil
catatan: per anak (persis `key` React) -- state anak DIPERTAHANKAN lintas render ulang induk
fungsi render_induk(props, state) {
    kembalikan "<div>" + komponen_anak(komp, "counter-1", {}) + "</div>"
}
```

**Filosofi Component System (disengaja):** render-ulang-penuh (HTML string
lewat `innerHTML`), BUKAN virtual-DOM diffing kayak React. Cukup buat skala
dashboard/CRUD, bukan pengganti diffing sungguhan buat UI sangat besar &
dalam. Event lewat atribut `data-aksi="nama"` (opsional
`data-peristiwa="input"`/`"change"`/`"submit"`/`"keyup"`, default `"click"`)
karena `render` cuma menghasilkan teks HTML, bukan pointer fungsi hidup —
handler aksi dapat `(props, state, event)`, nilai kembaliannya jadi state
baru (pola reducer). Nested components (`komponen_anak`) tetap render-ulang-
penuh di level ELEMEN DOM, tapi rekonsiliasi berbasis kunci di level
KOMPONEN, rekursif tanpa batas kedalaman — detail lengkap & trade-off di
`docs/KETERBATASAN.md`.

Contoh interaktif lengkap (buka `index.html` masing-masing lewat local
server, mis. `python3 -m http.server`):
- `runtime/web/contoh_router_state/` -- Router + State Management (navigasi antar "halaman", hitung kunjungan)
- `runtime/web/contoh_komponen/` -- Component System (Todo List: state, render, aksi, lifecycle hooks)
- `runtime/web/contoh_komponen_bersarang/` -- Nested/composed components (daftar komponen counter, satu instans per item, state independen & terjaga lintas render ulang induk)

**Temuan performa penting:** `isoteri-vm.js` TIDAK punya JIT (beda dari
native Rust yang punya Cranelift) -- diverifikasi langsung, `fib(38)` yang
selesai <5 detik di native masih belum selesai setelah 90 detik di browser.
Komputasi berat sebaiknya tetap di native/API, bukan langsung di browser.

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
