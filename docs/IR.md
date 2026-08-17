# Isoteri IR

## Apa yang jadi "IR" di Isoteri sekarang

`CStmt`/`CExpr` (dihasilkan oleh `Resolver` di `src/lib.rs`, bagian "3. RESOLVER")
diformalkan sebagai **Isoteri IR v1**. Ini bukan AST mentah (`Stmt`/`Expr` hasil
parser) lagi — sudah melalui proses resolve yang membuatnya backend-agnostic:

- Nama variabel → slot lokal/global konkret (`CExpr::Local(usize)` /
  `CExpr::Global(usize)`), bukan `String` yang perlu dicari lagi tiap backend.
- Akses field `bentuk` (struct) → sudah divalidasi & diurutkan sesuai skema.
- Closure → sudah dipisah jadi "fungsi bernama sintetis" + daftar ekspresi
  penangkap (`FungsiLiteral`), bukan lagi lexical scoping yang perlu
  ditelusuri ulang tiap backend.

Tiga backend yang ada — **Bytecode** (`Compiler`, bagian 5), **JIT native**
(`JitEngine`, bagian 5b), dan **web** (`ekspor_json_dari_sumber`) — semuanya
membaca representasi yang SAMA ini, bukan masing-masing menelusuri AST-nya
sendiri. Itulah yang membuatnya pantas disebut IR, bukan cuma "AST yang sudah
diresolve".

```text
        Stmt/Expr (AST mentah dari Parser)
                    │
                    ▼
              Resolver  ──────────────► Isoteri IR (CStmt/CExpr)
                                              │
                                    optimisasi_blok() / optimisasi_expr()
                                       (bagian "4b. ISOTERI IR: optimizer")
                                              │
                ┌─────────────────────────────┼─────────────────────────────┐
                ▼                             ▼                             ▼
            Compiler                     JitEngine                  ekspor_json_dari_sumber
          (bagian 5)                     (bagian 5b)                   (bagian 9)
                │                             │                             │
                ▼                             ▼                             ▼
            Instr (VM bytecode)         Kode mesin (Cranelift)      program.isoweb.json
                │                             │                             │
                ▼                             ▼                             ▼
          VM (native/AOT)              dipanggil langsung           isoteri-vm.js (browser/Node)
```

## Optimizer (bagian "4b" di `src/lib.rs`)

Karena semua backend membaca IR yang sama, optimisasi yang jalan SEKALI di IR
otomatis menguntungkan ketiga backend sekaligus — tidak perlu diimplementasikan
ulang per backend. v1 mencakup:

1. **Constant folding** (`lipat_binop`) — `2 + 3 * 4` dilipat jadi `14` saat
   kompilasi. Berlaku untuk Angka, Desimal (termasuk campuran Angka/Desimal),
   Teks (penggabungan `+`), dan Bool. Sengaja **konservatif soal pembagian**:
   `a / 0` TIDAK dilipat, supaya pesan error & nomor barisnya tetap identik
   dengan versi sebelum ada optimizer (optimizer tidak boleh mengubah perilaku
   yang teramati, cuma mempercepat).
2. **Dead code elimination** (bentuk paling sederhana, di `optimisasi_blok`) —
   statement setelah `kembalikan` pertama di blok yang sama dibuang, karena
   pasti tak terjangkau.

Diverifikasi: instruksi `PushK` tunggal menggantikan 3 instruksi `BinOp`
runtime untuk `2 + 3 * 4` (lihat contoh di riwayat percakapan/commit), dan
seluruh 16 program contoh di root proyek tetap menghasilkan output yang sama
persis sebelum & sesudah optimizer ini ditambahkan (diverifikasi silang lewat
perbandingan native vs `runtime/web/jalankan-node.js`, lihat
`runtime/web/README.md`).

Efek samping yang sengaja dijaga: fungsi yang kondisinya ikut terlipat jadi
`CExpr::Bool` literal (mis. `1 < 2`) TETAP dianggap elig untuk JIT
(`cek_jit_murni_kondisi` menerima `CExpr::Bool`) dan `JitEngine::kompilasi_kondisi`
tahu cara meng-codegen-kannya (`iconst`) — tanpa ini, optimizer yang "kepintaran"
justru bisa MENURUNKAN jumlah fungsi yang lolos JIT (regresi performa, walau
tetap benar secara semantik lewat jalur bytecode).

## IR Linear/Typed (v2, lanjutan di atas v1)

Lapisan BARU di antara IR v1 (pohon, bagian 4b) dan backend: representasi
**tiga-alamat, typed, register virtual** (`src/lib.rs` bagian "8b"). Ini
langkah menuju diagram `AST -> IR -> {Bytecode, JIT, AOT}` yang sebelumnya
cuma "IR pohon dibaca 3 backend terpisah" — sekarang ada representasi linear
eksplisit yang jadi prasyarat migrasi JIT dan SIMD/vectorization yang benar
(bukan ad-hoc seperti eksperimen SIMD yang direvert).

### Cara kerja singkat

- Register `0..local_slot_count` = slot lokal yang SUDAH ada (tanpa instruksi
  tambahan buat baca `CExpr::Local`). Register di atas itu = **temporary
  baru** yang dialokasikan selama lowering buat tiap hasil sub-ekspresi
  (di jalur stack lama ini implisit ada di stack; di sini eksplisit).
- Tipe (`IrType`) diturunkan bottom-up dari literal & `slot_tipe` yang sudah
  dihitung Resolver buat elig-JIT — BUKAN type-checker baru dari nol. Simpul
  yang tipenya belum bisa dipastikan (panggilan fungsi, daftar, peta, field)
  jatuh ke `IrType::Dinamis` (berlaku seperti `Value` biasa sekarang).
- Simpul yang belum "dilinearkan" murni (nested field-path assignment,
  `ulang selaras`) pakai **escape hatch** `IrInstr::Legacy` yang membungkus
  Instr hasil `Compiler` lama apa adanya (Hukum 6: "Explicit escape hatch" —
  jangan buru-buru menggeneralisasi kasus langka kalau manfaatnya kecil).
- Backend `ir_ke_instr_dgn_konstanta`: linear IR -> `Instr` (stack bytecode)
  biasa, supaya bisa langsung dijalankan & diuji lewat VM yang sudah ada.

### Status: BUKAN jalur produksi, tapi sudah diverifikasi benar

`jalankan_stmt_list_via_ir` / `isoteri via-ir program.iso` adalah jalur
**validasi**, dibandingkan byte-per-byte terhadap jalur produksi (metodologi
sama seperti validasi `runtime/web/`): **17/17 program contoh cocok persis**
(termasuk closure, struct, try/catch bersarang, callback struct-flattened,
dan bahkan pesan error jaringan yang sama persis di `program_coba_tangkap.iso`).

Dua bug nyata ditemukan & diperbaiki selama proses ini (bukti kenapa validasi
byte-per-byte penting, bukan cuma "kelihatannya benar"):
1. Nomor baris hilang (`TandaiBaris` tidak ikut di-lower) — semua pesan
   error jadi "Baris 0".
2. `param_flat` (dukungan callback struct-flattened untuk
   `petakan`/`saring`/`urutkan`) sempat di-hardcode kosong, bikin callback
   yang menerima instans `bentuk` gagal dengan pesan salah jumlah parameter.

### Overhead saat ini (jujur diukur, bukan diklaim)

**Update setelah register allocation v1** (destination-passing lowering —
lihat `IrLower::lower_expr_ke`/`reg_tujuan` di `src/lib.rs` bagian "8b"):
hasil `ingat`/`ubah` untuk **variabel lokal** (dalam fungsi) sekarang ditulis
LANGSUNG ke register tujuan, bukan ke temp lalu di-`Move` — pola
`BinOp(temp,...); Move(slot,temp)` yang sebelumnya muncul di SETIAP
assignment sekarang jadi `BinOp(slot,...)` langsung. `CobaLocal` juga
diperbaiki serupa (pesan galat ditulis langsung ke slot tangkap, bukan lewat
temp+Move — ternyata ini cocok persis dengan desain `Compiler` bytecode asli
yang memang tidak pernah butuh temp di situ, jadi sekaligus perbaikan bug
kecil, bukan cuma optimasi).

| Benchmark | Jalur lama | via-ir | Overhead |
|---|---|---|---|
| `loop_fungsi.iso` (loop di DALAM fungsi, variabel lokal) | 0.38s | 0.44s | **~15%** (dari ~2x sebelum optimasi ini) |
| `fib_rekursi.iso` (rekursi, parameter/lokal) | 0.19s | 0.34s | via-ir belum lewat JIT (disengaja, lihat poin 3 di bawah) |
| `loop_akumulasi.iso` (loop di TOP-LEVEL, variabel global) | 0.36s | 0.75s | **~2x -- BELUM membaik** (lihat penjelasan di bawah) |
| `daftar_petakan.iso` | 1.89s | 1.89s | Nyaris sama (didominasi alokasi Daftar, bukan overhead IR) |

**Temuan penting**: optimasi ini SPESIFIK menolong variabel **lokal**
(`IngatLocal`/`UbahLocal`), belum menolong variabel **global/top-level**
(`IngatGlobal`/`UbahGlobal`). Alasannya beda akar masalah: pembacaan
`CExpr::Global` SELALU butuh instruksi `LoadGlobal` ke suatu register (tidak
ada "akses langsung" seperti local slot), dan register itu lalu di-materialize
lewat `StoreLocal`/`LoadLocal` di backend `ir_ke_instr_dgn_konstanta` — bukan
dibiarkan di atas stack VM seperti compiler bytecode lama (yang murni
`LoadGlobal; LoadGlobal; BinOp; StoreGlobal`, 4 instruksi, TANPA slot lokal
sama sekali). Ini bukan soal "destination-passing" (yang sudah dibereskan),
tapi soal **stack scheduling** — lihat bagian di bawah.

## Stack Scheduling -- percobaan pertama SALAH, ditemukan lewat regresi, diperbaiki

Upaya pertama: post-pass di atas `Vec<Instr>` FINAL (bukan level IR linear
lagi, supaya tidak perlu mengurus reindexing target lompatan yang rumit) yang
menghapus pasangan `StoreLocal(r)` + `LoadLocal(r)` dengan celah SEMBARANG di
antaranya, asal tidak ada instruksi kontrol alur (`Lompat`/`LompatJikaSalah`/
iterasi/`coba`) di celah itu.

**Ini TERBUKTI SALAH.** Regresi 17/17 program tetap lolos (!) untuk kasus
non-rekursif, tapi begitu diuji dengan fungsi rekursif sederhana:

```isoteri
fungsi f(n) {
    kalau (n <= 1) { kembalikan n }
    kembalikan f(n - 1) + f(n - 2)
}
tampilkan f(10)
```

`f(10)` yang seharusnya `55` malah keluar `10` di jalur `via-ir`. Akar
masalahnya: kondisi "tidak ada kontrol alur di celah" ternyata TIDAK CUKUP.
Kalau di celah ada instruksi lain yang men-*push* nilai baru (mis.
`LoadLocal(0)` buat memuat operand LAIN dari `BinOp` yang sama), nilai `r`
yang "dibiarkan nangkring" di stack jadi ke-dahului nilai baru itu — untuk
`n <= 1`, urutan operand di stack diam-diam terbalik jadi `1 <= n`. Contoh
konkret yang ketangkep (dump instruksi sebelum/sesudah):

```
SEBELUM:  PushK(1); StoreLocal(1); LoadLocal(0); LoadLocal(1); BinOp(<=)
          -- stack saat BinOp: [n, 1] (benar: n <= 1)
SESUDAH (SALAH): PushK(1); LoadLocal(0); BinOp(<=)
          -- stack saat BinOp: [1, n] (terbalik: 1 <= n)
```

**17 program contoh yang ada TIDAK CUKUP buat menangkap bug ini** — bug baru
ketahuan lewat kasus rekursif yang secara kebetulan sensitif ke urutan
operand di celah semacam ini. Ini pelajaran nyata (bukan cuma teori) kenapa
validasi byte-per-byte penting tapi TIDAK CUKUP — perlu terus menambah kasus
uji begitu ada bug baru ditemukan, khususnya kasus dengan kontrol alur
bersarang (rekursi, dalam kasus ini).

**Perbaikan**: dipersempit jadi jauh lebih konservatif -- HANYA menghapus
pasangan yang **benar-benar bersebelahan** (`StoreLocal(r)` lalu PERSIS
instruksi berikutnya `LoadLocal(r)`, TANPA celah apa pun sama sekali). Ini
selalu aman: identik dengan "simpan lalu langsung ambil lagi tanpa ada apa-apa
terjadi di antaranya", tidak ada ruang buat instruksi lain menyerobot urutan
stack. Diverifikasi ulang: 17/17 program COCOK, termasuk kasus rekursif yang
tadi gagal.

| Benchmark | Jalur lama | via-ir (setelah perbaikan) | Overhead |
|---|---|---|---|
| `loop_fungsi.iso` (lokal) | 0.35s | 0.40s | ~15% (tidak berubah dari sebelum stack scheduling) |
| `fib_rekursi.iso` | 0.19s | 0.28s | via-ir belum lewat JIT (disengaja) |
| `loop_akumulasi.iso` (global) | 0.35s | 0.63s | ~1.8x (turun sedikit dari ~2x, TAPI masih jauh dari target) |
| `daftar_petakan.iso` | 2.02s | 1.93s | Nyaris sama |

**Kesimpulan jujur**: pendekatan post-pass "adjacent-only" TERBUKTI BENAR
tapi cakupannya kecil (kasus `loop_akumulasi` yang jadi target utama nyaris
tidak terbantu, karena pasangan Store/Load yang mahal di situ punya celah,
bukan bersebelahan — lihat contoh IR di bagian "Overhead saat ini" di atas).
Stack scheduling yang BENAR-BENAR menyelesaikan kasus itu butuh analisis
interferensi yang lebih hati-hati (lacak apakah instruksi di celah adalah
bagian dari sekuens PEMUATAN OPERAND untuk instruksi lain yang JUGA akan
mengonsumsi nilai kita — bukan cuma "ada kontrol alur atau tidak"). Ini
didokumentasikan sebagai kerja lanjutan terpisah, BUKAN diklaim selesai.

Ini **konsisten dengan tujuan v1**: buktikan dulu representasinya BENAR
(byte-per-byte identik, tetap 17/17 setelah optimasi ini), lalu optimasi
kecepatannya BERTAHAP dengan bukti terukur di tiap langkah -- bukan klaim
"udah dioptimasi" tanpa angka.

## Kerja lanjutan (belum dikerjakan, urutan disarankan)

1. **Dead-branch elimination** untuk `kalau` berkondisi konstan penuh
   (mis. `kalau (benar) { A } lainnya { B }` → cukup `A`). Sengaja ditunda di
   v1 karena butuh kehati-hatian pada penomoran baris (`TandaiBaris`) untuk
   pesan error yang terbuang.
2. ~~**IR yang benar-benar typed & linear**~~ — **SELESAI**, lihat "IR
   Linear/Typed (v2)" di atas. Register allocation-nya masih naif (belum ada
   dead store elimination / register reuse), itu jadi item lanjutan di poin
   berikutnya.
3. **Migrasi JIT** supaya generate kode dari IR linear yang sama (saat ini
   `JitEngine` masih menelusuri `CExpr`/`CStmt` sendiri secara terpisah dari
   `Compiler` MAUPUN dari IR linear yang baru) — ini jadi migrasi yang jauh
   lebih aman sekarang karena IR linear-nya sendiri sudah eksplisit &
   teruji byte-per-byte. Efek samping yang diharapkan: begitu ini selesai,
   overhead `via-ir` di tabel benchmark atas otomatis hilang untuk fungsi
   yang tadinya elig JIT.
4. **Register allocation** di `ir_ke_instr_dgn_konstanta` -- **SEBAGIAN
   SELESAI**: destination-passing untuk `ingat`/`ubah` variabel lokal (overhead
   loop-dalam-fungsi turun dari ~2x jadi ~15%) + stack scheduling versi
   konservatif (cuma pasangan Store/Load bersebelahan, lihat "Stack
   Scheduling" di atas -- termasuk cerita bug nyata yang ditemukan &
   diperbaiki). **BELUM SELESAI**: variabel top-level/global masih ~1.8-2x
   lebih lambat. Perbaikan lanjutan butuh stack scheduling yang lebih pintar
   (analisis interferensi: bedakan instruksi di celah yang "murni numpang
   lewat" vs yang jadi bagian dari sekuens pemuatan operand instruksi lain)
   -- BUKAN pekerjaan trivial mengingat bug yang sudah ditemukan di sini,
   jadi kalau dikerjakan lagi, WAJIB tambah kasus uji baru yang mengandung
   rekursi/kontrol alur bersarang, bukan cuma percaya validasi 17 program
   yang ada sekarang (terbukti tidak cukup sensitif).
5. ~~**Migrasi JIT**~~ -- **SELESAI (v1)**, lihat "Migrasi JIT ke IR Linear"
   di bawah.
5. **Backend AOT native langsung dari IR** (bukan lewat "generate proyek Rust
   lalu panggil `rustc`" seperti `isoteri bangun` sekarang) dan **backend WASM
   asli** begitu toolchain `wasm32-unknown-unknown` tersedia — dua-duanya
   akan jauh lebih mudah ditambahkan begitu poin 3 & 4 solid, karena tinggal
   menambah satu "backend baru" tanpa membongkar backend yang sudah ada.
6. **SIMD/vectorization** — baru masuk akal setelah poin 3 & 4 (IR linear
   dipakai nyata oleh JIT, bukan cuma bytecode), supaya compiler yang
   MEMUTUSKAN lewat analisis IR apakah vectorization untung, bukan
   "ada array → pakai SIMD" seperti eksperimen yang direvert.

Lihat `docs/FILOSOFI.md` untuk urutan milestone lengkap (Milestone A/B/C) dan
kenapa IR diprioritaskan sebelum package manager & DOM binding.

## AOT langsung dari IR

`isoteri bangun` (AOT) sebelumnya menghasilkan proyek Rust yang isinya
memanggil `isoteri::jalankan_sumber` (jalur lama, tree-walking) di
`main()`-nya -- artinya binary hasil build tetap lex/parse/resolve ULANG
setiap kali dijalankan, cuma dikemas jadi executable mandiri (bukan AOT
"sungguhan" dalam arti "kompilasi ke kode final sekali di waktu build").

Migrasinya sederhana justru KARENA fondasi IR linear sudah divalidasi
matang lewat `isoteri via-ir` sebelumnya: tinggal tambah
`pub fn jalankan_sumber_via_ir` (padanan `jalankan_sumber`, tapi memanggil
`jalankan_stmt_list_via_ir`) dan ubah template `main.rs` yang di-generate
`mode_bangun` (`src/main.rs`) supaya memanggil itu. Binary hasil `isoteri
bangun` sekarang menjalankan bytecode+JIT yang SAMA-SAMA generate dari IR
linear (bagian "8b"/"5b" di atas), bukan jalur tree-walking terpisah lagi.

**Kenapa aman dilakukan sebagai migrasi produksi langsung** (beda dari
`via-ir` yang sengaja dijaga sebagai jalur validasi terpisah): AOT sudah
sepenuhnya terisolasi jadi proses/binary lain lewat `cargo build` -- blast
radius kalau ada bug tidak menyentuh CLI `isoteri` utama sama sekali. Dan
fondasi yang dipakai (`jalankan_stmt_list_via_ir`) sudah melalui rangkaian
validasi panjang di bagian-bagian sebelumnya (17/17 regresi, kasus rekursif,
JIT-dari-IR yang terverifikasi CLIF-nya).

**Diverifikasi**: 17/17 program contoh di-build lewat `isoteri bangun` dan
outputnya dibandingkan terhadap jalur normal -- semua cocok. Performa
dibandingkan langsung (AOT lama vs AOT baru, `fib(38)`, 3x ulang masing-masing):

| | Run 1 | Run 2 | Run 3 |
|---|---|---|---|
| AOT lama (tree-walk) | 0.345s | 0.327s | 0.328s |
| AOT baru (IR linear) | 0.323s | 0.325s | 0.326s |

Nyaris identik -- konsisten dengan temuan migrasi JIT sebelumnya (setelah
koreksi kesalahan benchmark), tidak ada regresi performa dari migrasi ini.

### Kerja lanjutan AOT

- Saat ini AOT masih EMBED SUMBER TEKS `.iso` sebagai string constant di
  binary yang di-generate, dan tetap lex/parse/resolve saat program
  dijalankan (bukan cuma sekali saat `isoteri bangun`). Langkah lanjutan
  yang lebih "AOT sungguhan": serialisasi IR linear (mirip skema JSON yang
  dipakai `ekspor_json_dari_sumber` buat web) LANGSUNG ke binary sebagai data
  statis, sehingga proses jalan cukup deserialize lalu langsung eksekusi
  tanpa lex/parse/resolve/optimize ulang -- akan mempercepat waktu STARTUP
  (bukan throughput eksekusi, yang sudah ditunjukkan setara di tabel atas).
- Belum ada cara untuk AOT meng-compile fungsi non-JIT-eligible ke kode
  mesin native murni (fungsi yang bukan aritmatika murni tetap lewat
  bytecode VM yang di-embed) -- ini pada dasarnya sama dengan batasan JIT
  yang ada sekarang, bukan regresi baru dari migrasi AOT ini.

## Migrasi JIT ke IR Linear

`JitEngine::kompilasi_dari_ir` (`src/lib.rs` bagian "5b") adalah backend JIT
KEDUA, generate kode mesin dari `&[IrInstr]` (IR linear), berdampingan dengan
`JitEngine::kompilasi` (backend produksi, masih menelusuri `CExpr`/`CStmt`
langsung) -- BUKAN pengganti, dua-duanya hidup berdampingan lewat jalur
validasi `isoteri via-ir`, persis pola yang sama dipakai sepanjang dokumen
ini (bandingkan, jangan langsung ganti jalur produksi).

### Kenapa migrasinya relatif mulus (dibanding stack scheduling)

Elig-JIT (`cf.tipe_jit`, dihitung Resolver) **dipakai ulang apa adanya**,
tidak dihitung ulang dari IR -- fungsi yang lolos JIT produksi otomatis jadi
kandidat JIT-dari-IR juga, dengan jaminan subset instruksi yang SANGAT
terbatas (cuma `Const`/`Move`/`BinOp` non-Bagi/`PanggilFungsi`-ke-diri-sendiri
/`Jump`/`JumpJikaSalah`/`Kembalikan`/`TandaiBaris` -- semua kasus lain
`unreachable!()`, sama seperti pola pengaman yang sudah ada di
`KompilerBadan` produksi). Karena scope-nya sekecil ini, migrasi JIT
**TIDAK menemukan bug baru** lewat regresi 17/17 (kontras dengan stack
scheduling yang butuh 2 iterasi karena scope awalnya lebih ambisius/kurang
hati-hati) -- pelajaran dari situ (validasi lewat kasus rekursif, bukan cuma
percaya 17 program lama) tetap diterapkan di sini sebagai jaring pengaman,
cuma kebetulan tidak menangkap apa-apa kali ini.

### Arsitektur: basic block splitting dari IR linear ke Cranelift CFG

`IrInstr` punya target lompatan berupa INDEX ARRAY (bukan offset byte kayak
`Instr`), jadi pemetaan ke `Block` Cranelift relatif langsung: kumpulkan
semua "leader" (index 0, tiap target lompatan, index tepat setelah tiap
`Jump`/`JumpJikaSalah`), buat satu `Block` per leader, lalu jalan program-order
sambil `switch_to_block` tiap kali index sekarang adalah leader (nyisipkan
`jump` fallthrough eksplisit kalau block sebelumnya belum punya terminator).
Semua block SENGAJA tidak di-`seal` satu-satu, cuma `seal_all_blocks()` sekali
di akhir -- sama persis pola yang dipakai `kompilasi()` produksi, valid di
Cranelift selama semua block terisi sebelum `finalize()`.

### Optimasi kunci: register temporary TIDAK lewat mesin `Variable` Cranelift

Percobaan pertama (naif): setiap register -- baik local slot asli maupun
temporary hasil `IrLower` -- dideklarasikan sebagai Cranelift `Variable` dan
lewat `def_var`/`use_var`. Ini JALAN BENAR (lolos 17/17) tapi lebih lambat
dari JIT produksi (`fib(32)`: produksi 1.34s, JIT-dari-IR naif jauh lebih
lambat) -- sebabnya: Cranelift IR itu SENDIRI sudah SSA, `Variable` cuma
diperlukan buat menangani REASSIGNMENT lewat resolusi phi-node otomatis;
menaruh SETIAP hasil sub-ekspresi (yang dijamin ditulis sekali & dibaca
sekali oleh konstruksi `IrLower`, lihat bagian "8b") ke `Variable` adalah
kerja ekstra yang tidak perlu -- **kelas masalah yang SAMA PERSIS** dengan
overhead `StoreLocal`/`LoadLocal` berlebih yang dibereskan register
allocation v1 di backend bytecode, cuma versi Cranelift-nya.

Perbaikan: register `< ambang_temp` (local slot ASLI -- parameter + `ingat`
lokal, yang BISA di-reassign lintas block lewat kontrol alur) tetap lewat
`Variable` seperti biasa; register `>= ambang_temp` (temporary) di-cache
LANGSUNG sebagai nilai SSA mentah (`HashMap<Reg, cranelift::Value>`), tanpa
`Variable` sama sekali. Ini AMAN (dibuktikan lewat regresi, bukan cuma
diasumsikan) karena `IrLower` menjamin temporary tidak pernah dibaca dari
`Block` Cranelift yang beda dari `Block` tempat ia didefinisikan -- begitu
ada percabangan, register yang masih "in-flight" sudah pasti terkonsumsi
duluan oleh instruksi percabangan itu sendiri.

### Hasil, jujur diukur -- TERMASUK KOREKSI ATAS KESALAHAN BENCHMARK SENDIRI

**Koreksi penting**: angka "~35% overhead" yang tadinya dilaporkan di sini
**SALAH**, dan kesalahannya cukup mendasar: `benchmarks/fib_rekursi.iso`
ditulis sebagai `fungsi fib(n)` -- TANPA anotasi tipe (`n: Angka`). Ternyata
elig-JIT (`tipe_seragam` di Resolver, `src/lib.rs` bagian "3") mensyaratkan
SEMUA slot (parameter maupun lokal) punya anotasi tipe eksplisit yang
konsisten (`Some(TipeJit::Angka)` semua, atau `Some(TipeJit::Desimal)` semua)
-- parameter TANPA anotasi tipe otomatis `None`, jadi `tipe_seragam` jatuh ke
`None`, dan `cf.tipe_jit` juga `None`. Artinya `fib` **TIDAK PERNAH lolos
JIT sama sekali** -- baik di jalur produksi maupun `via-ir` -- sepanjang
"migrasi JIT" ini dikerjakan. Angka "39%"/"35% overhead" yang dilaporkan
sebelumnya sebenarnya membandingkan **bytecode lawan bytecode** (lewat
`ir_ke_instr_dgn_konstanta` + `stack_scheduling`), BUKAN JIT lawan JIT --
kesimpulan soal migrasi JIT-nya sendiri jadi tidak berdasar sama sekali.

Ini ditemukan lewat cara yang sama dipakai buat menangkap bug stack
scheduling sebelumnya: curiga terhadap angka yang tidak masuk akal, lalu
verifikasi LANGSUNG lewat dump CLIF (`ctx.func.display()`) alih-alih percaya
begitu saja pada hasil `time`. Dump CLIF buat `fib(n)` tanpa anotasi tipe
kosong sama sekali (fungsi `kompilasi()`/`kompilasi_dari_ir()` tidak pernah
dipanggil) -- itulah petunjuk pertama yang mengarah ke akar masalahnya.

Setelah diperbaiki (`fungsi fib(n: Angka)`, sama seperti
`program_jit.iso` yang sudah ada duluan di proyek ini dan seharusnya jadi
rujukan sejak awal), dump CLIF mengonfirmasi KEDUANYA benar-benar lolos JIT,
dan hasilnya:

| Benchmark (JIT sungguhan) | Produksi (`kompilasi`) | JIT-dari-IR (`via-ir`) |
|---|---|---|
| `fib(38)`, 3 kali ulang | 0.200s / 0.194s / 0.199s | 0.193s / 0.195s / 0.199s |

**Nyaris identik** (dalam margin noise pengukuran) -- migrasi JIT TIDAK
membawa regresi performa sama sekali, konsisten dengan perbandingan CLIF
manual: struktur block JIT-dari-IR malah SEDIKIT lebih ramping dari produksi
untuk kasus ini (produksi selalu membuat 3 block buat `kalau`/`kalau-lainnya`
bahkan tanpa klausa `lainnya`, termasuk satu block kosong yang cuma
`jump`; `via-ir` yang basic-block-splitting-nya berbasis leader position
tidak membuat block kosong itu).

**Pelajaran buat sesi berikutnya**: kalau menulis benchmark buat fitur
tertentu (JIT, SIMD, dst.), WAJIB verifikasi fitur itu SUNGGUH-SUNGGUH aktif
(lewat dump IR/CLIF/instruksi, bukan cuma percaya nama file atau asumsi),
sebelum mempercayai angka `time`. Ini kesalahan kedua dalam sesi ini setelah
bug stack scheduling -- keduanya ditemukan lewat sikap skeptis yang sama
("angka ini tidak masuk akal, mari saya buktikan"), bukan lewat proses yang
beda. Itu polanya, bukan kebetulan.

### Kerja lanjutan JIT

- ~~Telusuri sumber overhead ~35%~~ -- **TIDAK PERLU**, overhead itu tidak
  nyata (lihat koreksi di atas: kesalahan benchmark, bukan kesalahan
  codegen). JIT-dari-IR sudah setara performa dengan JIT produksi.
- Migrasikan `isoteri bangun` (AOT) supaya generate dari IR linear juga --
  saat ini AOT masih lewat jalur "generate proyek Rust lalu panggil rustc"
  yang sama sekali terpisah dari JIT.
- Perluas elig-JIT (`cf.tipe_jit`) sekarang ada IR linear yang lebih rapi
  buat dianalisis -- mis. dukungan `Bagi` (pembagian) dengan pengecekan
  div-by-zero eksplisit di codegen, bukan dihindari sama sekali.
- Tambahkan pemeriksaan otomatis di benchmark suite (`benchmarks/jalankan.sh`)
  yang MEMVERIFIKASI fitur yang sedang diukur benar-benar aktif (mis. cek
  keberadaan native fn pointer buat benchmark JIT), supaya kesalahan seperti
  di atas (benchmark JIT yang diam-diam tidak JIT) ketahuan otomatis lewat
  CI, bukan cuma lewat kecurigaan manual.
