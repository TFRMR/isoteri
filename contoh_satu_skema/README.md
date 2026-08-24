# Contoh: "Satu Skema, Dua Sisi"

Bukti konkret niche inti Isoteri (lihat bagian "Arah strategis" di
`ROADMAP.md`): **satu** definisi `bentuk` + fungsi validasi, dipakai
**identik** di backend (native) dan frontend (browser) -- tanpa disalin,
tanpa risiko dua aturan yang beda diam-diam (bug klasik di stack web
manapun: validasi form beda dari validasi API).

## Isi folder

- **`skema_petani.iso`** -- satu-satunya sumber kebenaran. `bentuk Petani`
  + fungsi `validasi_petani(data)`. Dipakai LANGSUNG (lewat `muat`) oleh
  `server.iso`, dan dipakai LEWAT `ekspor-web` (jadi
  `skema_petani.isoweb.json`) oleh `demo_satu_skema.html`.
- **`server.iso`** -- backend. `muat "skema_petani.iso"`, jalankan
  `server_mulai()`, validasi ulang data yang masuk sebelum "menyimpan"
  (mock in-memory, bukan database sungguhan -- ini contoh, bukan produk).
- **`skema_petani.isoweb.json`** -- hasil `isoteri ekspor-web
  skema_petani.iso` (sudah di-generate & di-commit, konsisten dengan
  contoh lain di repo ini -- lihat `.gitignore`). Regenerate kalau
  `skema_petani.iso` berubah:
  ```bash
  isoteri ekspor-web contoh_satu_skema/skema_petani.iso -o contoh_satu_skema/skema_petani.isoweb.json
  ```
- **`demo_satu_skema.html`** -- frontend. Form HTML biasa, validasi INSTAN
  di browser (manggil `validasi_petani()` langsung dari bundel di atas
  tiap kali user mengetik), lalu kalau valid betulan `fetch()` ke
  `server.iso` -- yang MEVALIDASI ULANG pakai fungsi yang sama persis.

## Cara jalankan

1. **Jalankan backend** (dari root repo):
   ```bash
   cd contoh_satu_skema
   isoteri server.iso
   ```
   Server jalan di `http://localhost:8899`. Biarkan terminal ini terbuka.

2. **Buka frontend** (terminal baru, dari root repo):
   ```bash
   cd contoh_satu_skema
   python3 -m http.server 8000
   ```
   Buka `http://localhost:8000/demo_satu_skema.html` di browser.

3. **Coba**:
   - Ketik nama, lahan, hasil panen -- validasi muncul instan di bawah
     form (dijalankan pakai `validasi_petani()` Isoteri yang SAMA persis
     dengan yang dipakai server, bukan JS terpisah yang ditulis manual).
   - Kosongkan nama, atau isi lahan negatif -- pesan error yang muncul
     PERSIS sama dengan yang akan dibalas server kalau data itu dipaksa
     kirim (coba lewat `curl`, bandingkan pesannya).
   - Isi data valid, klik "Validasi & Kirim ke Server" -- data betulan
     terkirim ke `server.iso` yang jalan di langkah 1, tersimpan di
     "database" in-memory-nya, dan responsnya ditampilkan.

## Kenapa ini penting (bukan cuma demo teknis)

Kalau besok aturan validasi berubah (mis. "hasil panen maksimal 500
ton"), cukup ubah **satu tempat** (`skema_petani.iso`), lalu
`ekspor-web` ulang. Frontend dan backend otomatis sinkron -- tidak ada
langkah "jangan lupa update juga validasi di form" yang gampang
terlewat di stack web biasa.

## Batasan contoh ini (disengaja, biar fokus)

- "Database"-nya cuma `Daftar` in-memory di `server.iso` -- hilang kalau
  server di-restart. Backend sungguhan tentu pakai database asli.
- CORS di `server_mulai()` permisif (`Access-Control-Allow-Origin: *`) --
  cukup buat demo lokal, backend produksi biasanya lebih ketat.
- Belum ada autentikasi/otorisasi -- di luar scope contoh ini.
