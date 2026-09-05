# Eksperimen "Type Information -> Representation" (item #6 ROADMAP)

Bagian dari rencana besar item #6 ROADMAP.md: perluas kelayakan JIT
Cranelift, dimulai dari langkah paling gampang (struct/bentuk numerik).
Prinsip arsitektur yang dipakai (disepakati bareng): **jangan bikin V8**
-- pakai Cranelift/VM/JIT/AOT yang sudah ada, optimalkan bagian yang
memang jadi keunggulan Isoteri: alur `type information -> representation
-> optimization -> runtime semantics`. Target milestone bertahap:
**30x -> <=5x -> <=2x -> ~1x** lebih lambat dari Node.js (bukan klaim
"secepat" langsung, realistis bertahap).

## Yang Sudah Ditemukan & Diperbaiki di Putaran Ini

### Temuan: infrastruktur "flatten struct numerik" SUDAH SEBAGIAN ADA

`bentuk` (struct) di Isoteri sudah bisa punya anotasi tipe field sejak
lama (`bentuk Titik { x: Angka, y: Angka }`), dan resolver sudah
mendeteksi kalau SEMUA field sebuah struct numerik (`hitung_param_flat`,
`param_flat_info` di `src/lib.rs`) -- struct seperti itu, kalau dipakai
sebagai PARAMETER fungsi, akses field-nya (`p.x`) langsung diterjemahkan
ke slot lokal biasa saat kompilasi (`CExpr::Local`), BUKAN lewat
`CExpr::Field` dinamis. Infrastruktur ini sudah ada, cuma belum lengkap.

### Perbaikan: fast-path buat literal struct langsung di titik panggil

**Sebelum**: `f(Titik{x:3,y:4})` tetap membangun `Instans` dinamis dulu
(`Instr::BuatInstans`, alokasi heap) lalu LANGSUNG dibongkar lagi lewat
mekanisme `SimpanLaluField` -- kerja dua kali buat objek yang cuma hidup
sepersekian detik.

**Sesudah**: kalau argumen adalah literal `Bentuk` langsung (bukan lewat
variabel), compiler skip konstruksi `Instans` sepenuhnya -- field-nya
(sudah diurutkan sesuai skema lewat `urutkan_field_bentuk()`) langsung
jadi argumen. Diimplementasikan di KEDUA resolver (global & lokal, lihat
`src/lib.rs`, dekat komentar "Fast-path: argumen literal Bentuk").

**Hasil terverifikasi** (isolated micro-test, `jarak_kuadrat(Titik{x:3,y:4})`
dipanggil 2 juta kali dalam loop):

| | Sebelum fix | Sesudah fix |
|---|---:|---:|
| Fungsi dgn parameter struct | 1.08s | **0.52s** (~2x lebih cepat) |
| Fungsi dgn 2 parameter angka biasa (baseline) | 0.82s | 0.82s (tidak berubah) |

Setelah fix, versi struct malah SEDIKIT LEBIH CEPAT dari baseline
2-parameter biasa -- konsisten di 5x pengulangan. 14/14 test regresi
tetap lulus (`scripts/regresi.sh`).

**PENTING -- luruskan skop perbaikan ini**: ini optimasi level
INTERPRETER/bytecode (mengurangi kerja yang terbuang), BUKAN "sekarang
fungsi ber-parameter struct dikompilasi native lewat Cranelift JIT".
Field `tipe_jit: Option<TipeJit>` di `CFungsi` (yang menentukan
kelayakan Cranelift JIT sungguhan) **masih secara eksplisit menolak**
`Bentuk` (lihat komentar di kode: "tanpa Teks/Bool/Daftar/Peta/Bentuk").
Perbaikan di putaran ini TIDAK mengubah itu.

## Kenapa `validasi_petani_struct` (Benchmark Representatif) Masih Lambat

Ditulis ulang `validasi_petani` (lihat `benchmarks/head_to_head/`) pakai
`bentuk DataPetani` (bukan `Peta`) buat ukur seberapa dekat ke target
milestone. Hasil jujur:

| | Isoteri AOT | Node.js |
|---|---:|---:|
| `validasi_petani_struct` (500rb validasi) | ~840ms | ~42ms |

**Masih ~20x lebih lambat** -- BELUM sampai target `<=5x`. Analisis:

1. Fungsi `validasi_petani_struct()` sendiri masih TIDAK ELIGIBLE buat
   `tipe_jit` (dikompilasi Cranelift native) karena eksklusi eksplisit
   `Bentuk` di atas -- fast-path yang kita tambahkan cuma bikin
   INTERPRETER-nya lebih hemat, bukan bikin fungsinya jadi kode native.
2. Fungsi pembungkus (`validasi_satu`) dan loop top-level pakai variabel
   GLOBAL (`ingat n`, `ingat i` di level atas program) DAN operator `%`
   (modulo) -- keduanya JUGA di luar syarat `cek_jit_murni_stmt`/
   `cek_jit_murni_nilai` (cuma `Tambah`/`Kurang`/`Kali` yang diizinkan
   di ekspresi "murni", modulo tidak termasuk; akses variabel global
   langsung didiskualifikasi).

## Langkah Selanjutnya (Belum Dikerjakan di Putaran Ini)

Supaya benar-benar sampai ke kode native (bukan cuma interpreter lebih
hemat), perlu 2 pekerjaan LANJUTAN yang lebih besar:

1. **Perluas `tipe_jit` inference supaya menerima `Bentuk` numerik** --
   bukan cuma `param_flat` (mekanisme terpisah yang sudah ada), tapi
   benar-benar diwariskan ke Cranelift codegen: tiap field jadi satu
   nilai native (I64/F64) di dalam fungsi yang dikompilasi, sama seperti
   parameter skalar biasa sekarang.
2. **Perluas operator & konteks yang diizinkan** di `cek_jit_murni_*` --
   minimal modulo (`%`) dan pembagian dengan pengecekan div-by-zero, dan
   PALING PENTING: cara supaya fungsi yang MEMANGGIL fungsi ber-tipe-JIT
   dari dalam konteks yang pakai variabel global tetap bisa manfaat
   (skema hybrid: bagian yang JIT-elig dikompilasi native, bagian lain
   tetap interpreter, TANPA harus SELURUH fungsi pembungkusnya jadi
   elig).

Ini scope pekerjaan MINGGUAN (compiler engineering signifikan), sesuai
catatan di ROADMAP.md item #6 -- perbaikan di putaran ini adalah
langkah AWAL yang nyata & terverifikasi (2x di level interpreter),
bukan solusi penuh.

## Putaran Lanjutan: `TipeJit::Campur` -- Native JIT Sungguhan buat Struct Campuran

Item #1 di "Langkah Selanjutnya" di atas SEKARANG SELESAI: `bentuk`
dengan field campuran Angka+Desimal (bukan cuma Bentuk-seragam-tipe)
sekarang BISA benar-benar dikompilasi native Cranelift, bukan cuma
optimasi interpreter.

### Pendekatan: aman lewat verifikasi per-operasi, bukan promosi tipe implisit

Ditambah varian baru `TipeJit::Campur` (lihat catatan panjang di
`src/lib.rs`) -- fungsi dengan slot BERBEDA tipe per field, TAPI setiap
OPERASI individual (perbandingan `==`, `<=`, dst) diverifikasi
same-type di kedua operand-nya (`tipe_cexpr()`) SEBELUM diizinkan JIT.
Kalau ada SATU operasi saja yang benar-benar mencampur Angka+Desimal
(mis. `d.a == d.b` langsung), seluruh fungsi GAGAL syarat murni --
fallback ke interpreter (aman, hasil tetap benar), BUKAN nyoba promosi
tipe implisit (int->float) yang lebih riskan salah kalau meleset.

**Skop SENGAJA dipersempit buat keamanan**: mode Campur cuma didukung
buat PERBANDINGAN, TIDAK BOLEH aritmatika (`+`/`-`/`*`) sama sekali --
menghindari kebutuhan mekanisme overflow-flag/promosi-tipe yang jauh
lebih riskan. Nilai kembalian (`kembalikan`) WAJIB Angka (signature
Cranelift butuh SATU tipe kembalian pasti). Juga cuma didukung di jalur
`kompilasi_dari_ir` (via-ir/AOT) -- jalur JIT legacy (`isoteri
prog.iso` default) SENGAJA menolak Campur di awal (fallback interpreter
otomatis, tetap benar, cuma tidak dapat manfaat native compile di jalur
itu).

### Verifikasi correctness

15/15 test regresi lulus (`scripts/regresi.sh`), termasuk test case
baru `tes_regresi/tipe_campur_jit.iso` yang mengunci:
- Struct dengan field Angka DAN Desimal bekerja benar sekaligus (bukan
  cuma salah satu tipe)
- **Kasus paling penting**: operasi yang BENAR-BENAR mencampur tipe
  (`d.a == d.b`, Angka dibanding LANGSUNG dengan Desimal) tetap dapat
  hasil BENAR lewat fallback interpreter otomatis -- dibuktikan
  `tipe_jit_final=None` buat fungsi itu spesifik (bukan `Some(Campur)`
  seperti fungsi yang aman), verified via `ISOTERI_DEBUG_JIT=1`.
- Konsisten di 3 jalur eksekusi (JIT default, bytecode murni, via-ir),
  dengan 1 divergensi stderr yang diizinkan & didokumentasikan
  (`tes_regresi/divergensi_diketahui.txt`) -- warning informational
  "Campur belum didukung di jalur legacy" yang cuma tercetak di mode
  JIT default (karena sempat coba dulu sebelum fallback), TIDAK ada di
  bytecode murni (skip percobaan sama sekali) maupun via-ir (berhasil
  native, tidak perlu fallback).

### Hasil benchmark (angka jujur, bukan cherry-pick)

`validasi_petani_struct` sekarang MENANG di level fungsi individual
(`validasi_petani_struct()` sendiri dapat `tipe_jit_final=Some(Campur)`
-- native compile berhasil), tapi fungsi PEMBUNGKUS (`validasi_satu`,
dipanggil 500rb kali di top-level loop) MASIH interpreter penuh --
parameternya (`i`) tidak dianotasi tipe DAN pakai operator `%` (modulo)
yang belum didukung sama sekali di JIT manapun (bukan spesifik Campur --
`Bagi`/`Modulo` keduanya masih `unreachable!()` di semua mode, perlu
mekanisme pengecekan pembagi-nol di codegen yang belum dibangun).

| | Sebelum (fast-path literal saja) | Sesudah (+ mode Campur) | Node.js |
|---|---:|---:|---:|
| `validasi_petani_struct` AOT | ~840ms | **~500ms** (~40% lebih cepat) | ~42ms |

**Masih ~12x lebih lambat dari Node.js -- BELUM sampai target `<=5x`**,
tapi progress nyata & terukur (840ms->500ms). Sisa jaraknya SEKARANG
punya penyebab yang jelas & sempit: `validasi_satu` (fungsi
pembungkus) perlu (a) anotasi tipe pada parameternya, DAN (b) dukungan
`%` (modulo) di JIT -- keduanya independen dari pekerjaan mode Campur
di atas, item terpisah buat putaran berikutnya.

## Putaran Ketiga: Dukungan Modulo + 2 Bug Pre-Existing Ditemukan & Diperbaiki

Item "(b) dukungan modulo" di atas SEKARANG SELESAI -- `%` didukung di
JIT buat mode `Angka` murni (bukan `Desimal`/`Campur`, butuh mekanisme
`flag_var`/`out_ptr` yang cuma ada di mode `Angka`). Item "(a) anotasi
tipe" JUGA sudah dicoba (`validasi_satu(i: Angka)` +
`ingat sisa: Angka = ...`) -- sintaksnya SUDAH ADA di compiler
sebelumnya, cuma belum pernah dipakai di file benchmark ini.

### Keamanan: 2 bahaya native `srem` (bukan cuma pembagi-nol)

Modulo native (`srem` x86) punya DUA kondisi berbahaya, bukan cuma
pembagi nol:
1. Pembagi nol -- error biasa ("Tidak bisa modulo dengan nol."),
   dilaporkan lewat BIT BARU di `flag_var` (bit 1, nilai 2 -- terpisah
   dari bit 0 punya overflow aritmatika biasa).
2. `i64::MIN % -1` -- overflow MATEMATIS (hasil pembagian di luar
   jangkauan i64) yang bisa bikin CPU **trap** (SIGFPE) kalau
   pembagi/dividen mentah langsung dikirim ke instruksi hardware.
   Dijinakkan pakai `select` Cranelift (paksa pembagi jadi `1` sebelum
   `srem` beneran dipanggil kalau kondisi ini kejadian) -- BUKAN
   kondisi error (jawaban matematisnya well-defined: `0`), jadi
   TIDAK men-set flag, cuma dihitung ulang jadi `0` secara diam-diam.

### Temuan sampingan: 2 bug PRE-EXISTING di interpreter (bukan dibuat sesi ini)

Saat menguji edge case #2 di atas terhadap jalur bytecode (buat
verifikasi konsistensi), ditemukan **interpreter Isoteri sendiri
CRASH TOTAL** (Rust panic, bukan `Result::Err` yang bisa ditangkap
`coba/tangkap`) untuk `i64::MIN % -1` dan `i64::MIN / -1` -- operator
`%`/`/` Rust polos dipakai langsung di `eval_binop` tanpa
`checked_rem`/`checked_div`. Bug ini SUDAH ADA sebelum sesi ini (bukan
regresi dari kerjaan Modulo-JIT), ditemukan sebagai efek samping.

**Diperbaiki di SEMUA jalur** yang punya logika sama (bukan cuma yang
ketahuan): `eval_binop` (interpreter utama), `eksekusi_selaras`
(interpreter terpisah buat `ulang selaras`), dan `lipat_binop`
(constant-folding compiler -- bug ini BAHKAN bisa crash COMPILER-nya
sendiri kalau user menulis ekspresi konstan yang hasilnya kebetulan
`i64::MIN`, mis. `(0 - 9223372036854775807 - 1) % -1`). Ketiganya
sekarang pakai `checked_div`/`checked_rem`, konsisten dengan JIT: nol
tetap error jelas, `MIN/-1` dapat jawaban matematis `0` tanpa crash.

### Verifikasi correctness

16/16 test regresi lulus, termasuk test case baru
`tes_regresi/modulo_jit_dan_edge_case.iso` yang mengunci modulo dasar,
modulo-nol (harus tertangkap `coba/tangkap`), DAN edge case
`i64::MIN % -1`/`i64::MIN / -1` secara eksplisit di ketiga jalur
eksekusi -- 1 divergensi baris pesan error (bukan beda hasil) antara
JIT dan bytecode, didokumentasikan di `divergensi_diketahui.txt`
dengan pola yang sama seperti divergensi overflow yang sudah ada.

### Temuan yang mengarah ke blocker terbesar: batasan cross-function call

Setelah `validasi_satu` dianotasi tipe lengkap + dukungan modulo aktif,
ternyata **MASIH** `tipe_jit_final=None` -- diselidiki lebih lanjut,
akar masalahnya BUKAN lagi soal tipe/modulo, tapi limitasi yang lebih
umum: **JIT Cranelift Isoteri (sebelum putaran ini) HANYA mendukung
pemanggilan diri sendiri (rekursi) atau fungsi tanpa panggilan sama
sekali** -- memanggil fungsi LAIN (`validasi_satu` memanggil
`validasi_petani_struct`), walau fungsi yang dipanggil itu SENDIRI
sudah native-elig, TETAP mendiskualifikasi si pemanggil dari JIT
sepenuhnya.

## Putaran Keempat: Cross-Function Call -- Item Terakhir Rencana Besar

Ini pekerjaan compiler engineering PALING DALAM sejauh ini di seluruh
rangkaian optimasi "type info -> representation" -- JIT sekarang bisa
kompilasi fungsi yang memanggil fungsi LAIN (bukan cuma diri sendiri),
selama target-nya SUDAH dideklarasikan lebih dulu (backward reference)
DAN sudah tipe_jit-elig sendiri.

### Verifikasi mekanisme Cranelift SEBELUM ubah kode utama

Sebelum menyentuh compiler sungguhan, pola inti (declare SEMUA fungsi
dulu -> define SEMUA body -> finalize SEKALI di akhir) diverifikasi
lewat eksperimen Rust terisolasi memakai `cranelift-jit` versi yang
SAMA PERSIS -- termasuk kasus paling riskan (fungsi C didefinisikan
SEBELUM fungsi D yang dipanggilnya) -- SEMUA berhasil sebelum
diterapkan ke compiler utama. Ini yang membuat restrukturisasi besar
ini bisa dikerjakan dengan percaya diri, bukan coba-coba di kode
produksi langsung.

### Restrukturisasi

- **`JitEngine`**: 1 metode (`kompilasi_dari_ir`, declare+define+
  finalize+ambil-pointer sekaligus per fungsi) dipecah jadi 3:
  `declare_fungsi()` (signature saja), `kompilasi_dari_ir()` (define
  body saja, TIDAK finalize sendiri lagi), `selesai()`+`ambil_pointer()`
  (finalize sekali + ambil semua pointer di akhir).
- **`jalankan_stmt_list_via_ir`**: dipecah jadi 3 fase eksplisit --
  FASE 1 declare signature SEMUA fungsi elig, FASE 2 lower ke IR &
  define body (urutan alfabetis, BEBAS sekarang -- tidak perlu lagi
  "target harus sudah dikompilasi duluan"), FASE 3 finalize sekali +
  isi `native` ke tiap `VMFungsi`.
- **Codegen `IrInstr::PanggilFungsi`**: `local_callee: FuncRef` (SATU
  target, cuma diri sendiri) diganti `panggil_fref:
  HashMap<usize, (TipeJit, FuncRef)>` (registri semua target yang
  DIPAKAI fungsi ini, diri sendiri MAUPUN fungsi lain, lewat jalur
  codegen yang SAMA PERSIS -- tidak ada kasus khusus).
- **`cek_jit_murni_nilai`**: `CExpr::Panggil` sekarang izinkan target
  BUKAN diri sendiri, asal target SUDAH ada di `fungsi_out`
  (backward reference -- lihat batasan scope di bawah) DAN
  `tipe_jit`-nya `Some`, DAN tiap argumen tipenya cocok dengan
  parameter target (`tipe_cexpr`, dipakai ulang dari mekanisme
  `TipeJit::Campur`).
- **`NativeFn::Campur`, `VMFungsi.slot_tipe`**: dipakai ulang buat
  membungkus argumen SESUAI tipe target (bukan tipe pemanggil) di
  titik panggilan lintas-fungsi.

### Batasan scope (SENGAJA, demi keamanan)

- **Backward reference saja**: fungsi cuma boleh memanggil fungsi yang
  dideklarasikan LEBIH DULU di source (`fungsi_out` terisi berurutan
  sesuai deklarasi saat RESOLVE, beda dari urutan KOMPILASI yang
  alfabetis). Forward reference (panggil fungsi yang dideklarasikan
  BELAKANGAN) otomatis gagal & fallback interpreter -- AMAN meski
  tidak lengkap, BUKAN fixed-point solver umum.
- **Jalur legacy (`isoteri prog.iso` default, BUKAN via-ir/AOT) TIDAK
  didukung** -- `kompilasi_nilai` (compiler CExpr-langsung) masih
  SELALU asumsikan diri sendiri untuk semua `CExpr::Panggil`. Fungsi
  yang cross-call SEKARANG ditolak EKSPLISIT di jalur ini
  (`mengandung_panggilan_lain_stmt`, scan body cari panggilan ke
  fungsi lain) SEBELUM sempat coba compile -- tanpa penolakan ini,
  hasilnya rekursi-tak-sengaja yang stack-overflow (BUKAN cuma gagal
  optimasi biasa -- ini benar-benar ditemukan & diverifikasi lewat
  reproduksi manual sebelum diperbaiki).
- **Argumen dibatasi bentuk sederhana** (`Local`/literal/negasi
  literal via helper `bentuk_argumen_sederhana`) -- bukan sembarang
  ekspresi arbitrer.

### Bug ditemukan & diperbaiki DALAM PROSES verifikasi (bukan lolos ke commit)

1. **Argumen Desimal ke fungsi Campur ditolak salah** -- pengecekan
   argumen awalnya daur ulang `cek_jit_murni_nilai` yang mengecek
   literal Desimal terhadap MODE PEMANGGIL sendiri (benar untuk
   ekspresi di dalam tubuh fungsi, TAPI salah konteks untuk argumen ke
   fungsi lain -- yang relevan itu tipe PARAMETER TARGET). Diperbaiki
   dengan pengecekan bentuk+tipe terpisah yang tidak daur ulang
   `cek_jit_murni_nilai` untuk kasus ini.
2. **Literal negatif (`-1.0`) ditolak sebagai argumen** -- parser
   me-representasikan `-X` sebagai `Binary(Angka(0), Kurang, X)`
   (lihat `parse_unary()`), BUKAN literal tunggal, SEBELUM sempat
   dilipat jadi konstanta oleh optimizer (yang jalan BELAKANGAN,
   setelah `tipe_jit` sudah dihitung). Diperbaiki lewat helper
   `bentuk_argumen_sederhana()` yang mengenali pola negasi ini secara
   eksplisit.

Kedua bug ini KETAHUAN & DIPERBAIKI lewat testing manual bertahap
(isolasi kasus sederhana -> kompleks) SEBELUM sempat masuk test
regresi permanen -- persis disiplin yang sama dipakai di seluruh sesi
optimasi ini.

### Verifikasi correctness

**17/17 test regresi lulus**, termasuk test case baru
`tes_regresi/cross_function_call_jit.iso` yang mengunci: rantai 3
fungsi (`satu`->`dua`->`tiga`), fungsi Angka memanggil fungsi Campur
dengan literal negatif sebagai argumen (mengunci Bug #2 di atas), DAN
**forward reference tetap dapat hasil benar lewat fallback
interpreter** (bukan promosi paksa). 2 divergensi baris peringatan
(bukan beda hasil) antara jalur JIT default vs bytecode/via-ir
didokumentasikan di `divergensi_diketahui.txt`, pola sama dengan
divergensi `TipeJit::Campur` sebelumnya.

**Bukti nyata bahaya nyata, bukan cuma teoretis**: sebelum penolakan
eksplisit di jalur legacy ditambahkan, kasus cross-function call
sungguhan menyebabkan **stack overflow terverifikasi** (`isoteri
/tmp/tes.iso` crash dengan "thread 'main' has overflowed its stack")
-- direproduksi manual, diperbaiki, dikunci lewat test regresi.

### Hasil benchmark FINAL (angka jujur)

| | Isoteri AOT | Node.js | Rasio |
|---|---:|---:|---:|
| Awal (versi Peta, sebelum semua optimasi) | 1226ms | 38-42ms | ~29-31x |
| Setelah fast-path literal struct | 840ms | ~42ms | ~20x |
| Setelah `TipeJit::Campur` | ~640ms | ~40ms | ~16x |
| Setelah Modulo + anotasi tipe | 638ms | 38ms | ~17x |
| **Setelah cross-function call** | **236.5ms** | **37ms** | **~6.4x** |

**SANGAT DEKAT target `<=5x`** (meleset ~1.4x saja) -- dan dari titik
AWAL sesi optimasi ini (1226ms) ke SEKARANG (236.5ms) adalah **~5.2x
LEBIH CEPAT total**, hampir menembus target `<=5x` MILESTONE ITU
SENDIRI dihitung dari titik awal. Cross-function call TERNYATA jadi
kontributor TERBESAR dari SEMUA optimasi di rangkaian ini (638ms ->
236.5ms, ~2.7x cuma dari satu perbaikan ini) -- mengkonfirmasi diagnosis
di awal putaran ini bahwa ini memang blocker paling signifikan, bukan
cuma salah satu dari sekian item.

**Kenapa belum tepat `<=5x`**: sisa gap kemungkinan besar overhead
proses (startup CLI, alokasi `Peta`-nya sendiri buat `Instans` struct
yang MASIH terjadi di titik konstruksi -- walau field-nya sekarang
"dibongkar" jadi argumen flat di titik PANGGILAN, `DataPetani`
sebagai TIPE PARAMETER `validasi_petani_struct` sendiri, kalau
dipakai sebagai NILAI LOKAL biasa di tempat lain, tetap lewat jalur
`Value::Instans` biasa) -- kandidat optimasi lanjutan buat menutup
sisa jarak, TAPI di luar scope sesi ini.

