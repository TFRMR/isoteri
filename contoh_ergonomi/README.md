# Contoh Ergonomi Bahasa

Program `.iso` kecil yang mendemonstrasikan fitur ergonomi bahasa yang
ditambahkan dalam beberapa sesi pengembangan terakhir. Tiap file fokus ke
satu/dua fitur terkait, jalankan langsung buat lihat perilakunya:

```bash
isoteri contoh_ergonomi/else_if_dan_loop_kontrol.iso
```

| File | Fitur yang didemokan |
|---|---|
| `else_if_dan_loop_kontrol.iso` | `lainnya kalau` (else-if), `putus`/`lanjut` di `ulang`/`ulang setiap` |
| `putus_lanjut_di_dalam_coba.iso` | `putus`/`lanjut` yang melompat keluar dari dalam blok `coba/tangkap` di tengah loop |
| `loop_bersarang_putus_lanjut.iso` | `putus`/`lanjut` di loop bersarang -- selalu ke loop terdekat |
| `modulo_dan_compound_assignment.iso` | Operator `%`, `+=`/`-=`/`*=`/`/=`, `++`/`--` |
| `assignment_lewat_indeks.iso` | `daftar[0] = x`, `peta["k"] = x`, nested (`matriks[0][1] = x`), campur field+indeks |
| `closure_di_petakan_saring_urutkan.iso` | Closure inline & dengan capture sebagai callback `petakan`/`saring`/`urutkan` |
| `negasi_boolean.iso` | Operator `!` (negasi boolean, pakai truthiness sama seperti `kalau`) |

Detail lengkap tiap fitur, termasuk batasannya, ada di
[docs/REFERENSI.md](../docs/REFERENSI.md) dan
[docs/KETERBATASAN.md](../docs/KETERBATASAN.md).
