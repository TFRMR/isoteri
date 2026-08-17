use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 && args[1] == "bangun" {
        if let Err(e) = mode_bangun(&args[2..]) {
            eprintln!("Kesalahan Bangun: {}", e);
            std::process::exit(1);
        }
        return;
    }

    if args.len() > 1 && args[1] == "ekspor-web" {
        if let Err(e) = mode_ekspor_web(&args[2..]) {
            eprintln!("Kesalahan Ekspor Web: {}", e);
            std::process::exit(1);
        }
        return;
    }

    if args.len() > 1 && args[1] == "via-ir" {
        // Jalur VALIDASI, bukan produksi -- lihat docs/IR.md poin 2. Menjalankan program lewat
        // pipeline IR LINEAR (typed, tiga-alamat) yang baru, buat dibandingkan byte-per-byte
        // terhadap jalur biasa: `diff <(isoteri p.iso) <(isoteri via-ir p.iso)` harus kosong.
        let path = args.get(2).cloned().unwrap_or_else(|| "program.iso".to_string());
        if let Err(e) = isoteri::jalankan_berkas_via_ir(&path) {
            eprintln!("{}", e);
            std::process::exit(1);
        }
        return;
    }

    if args.len() > 1 && args[1] == "init" {
        if let Err(e) = mode_init(&args[2..]) {
            eprintln!("Kesalahan Init: {}", e);
            std::process::exit(1);
        }
        return;
    }

    if args.len() > 1 && args[1] == "tambah" {
        if let Err(e) = mode_tambah(&args[2..]) {
            eprintln!("Kesalahan Tambah: {}", e);
            std::process::exit(1);
        }
        return;
    }

    if args.len() > 1 && args[1] == "uji" {
        let kode = mode_uji(&args[2..]);
        std::process::exit(kode);
    }

    if args.len() > 1 && args[1] == "format" {
        let kode = mode_format(&args[2..]);
        std::process::exit(kode);
    }

    let path = if args.len() > 1 {
        args[1].clone()
    } else if Path::new("isoteri.toml").exists() && Path::new("src/main.iso").exists() {
        // Default cerdas: kalau ini proyek berbasis isoteri.toml (lihat `isoteri init`),
        // entry point bawaannya src/main.iso -- bukan lagi program.iso apa adanya.
        "src/main.iso".to_string()
    } else {
        "program.iso".to_string()
    };
    if let Err(e) = isoteri::jalankan_berkas(&path) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

/// Subcommand `isoteri init [nama]` -- bikin proyek baru minimal: `isoteri.toml` +
/// `src/main.iso`. `nama` opsional, default nama direktori kerja saat ini.
fn mode_init(argv: &[String]) -> Result<(), String> {
    if Path::new("isoteri.toml").exists() {
        return Err("Sudah ada isoteri.toml di direktori ini -- \"isoteri init\" tidak akan menimpa.".to_string());
    }
    let nama = argv.first().cloned().unwrap_or_else(|| {
        env::current_dir().ok()
            .and_then(|d| d.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| "aplikasi-isoteri".to_string())
    });
    fs::write("isoteri.toml", format!("nama = \"{}\"\nversi = \"0.1.0\"\n\n[dependensi]\n# lokal: isoteri tambah nama_paket path/ke/paket\n# registry (git): isoteri tambah nama_paket --git URL --tag v1.0.0\n", nama))
        .map_err(|e| format!("Gagal menulis isoteri.toml: {}", e))?;
    fs::create_dir_all("src").map_err(|e| format!("Gagal membuat direktori src/: {}", e))?;
    if !Path::new("src/main.iso").exists() {
        fs::write("src/main.iso", "tampilkan \"Halo dari Isoteri!\"\n")
            .map_err(|e| format!("Gagal menulis src/main.iso: {}", e))?;
    }
    eprintln!("Proyek \"{}\" dibuat. Jalankan lewat: isoteri", nama);
    Ok(())
}

/// Subcommand `isoteri tambah` -- daftarkan dependensi ke [dependensi] isoteri.toml, dua bentuk:
/// - LOKAL:  `isoteri tambah nama_paket path/ke/paket`
/// - GIT (registry v1, lihat docs/FILOSOFI.md Milestone C):
///   `isoteri tambah nama_paket --git URL --tag v1.0.0`
///   `isoteri tambah nama_paket --git URL --rev <commit_hash>`
/// Paket tujuan harus punya src/lib.iso (konvensi entry point buat dijadikan dependensi,
/// beda dari src/main.iso yang buat dijalankan langsung). Untuk dependensi git, paket
/// langsung diambil (git clone) saat ini juga -- gagal cepat kalau URL/tag/rev salah,
/// daripada baru ketahuan nanti pas `muat` dipanggil.
fn mode_tambah(argv: &[String]) -> Result<(), String> {
    const PAKAI: &str = "pakai:\n  isoteri tambah nama_paket path/ke/paket\n  isoteri tambah nama_paket --git URL --tag v1.0.0\n  isoteri tambah nama_paket --git URL --rev <commit_hash>";
    let nama_paket = argv.first().ok_or(PAKAI)?;

    if !Path::new("isoteri.toml").exists() {
        return Err("Tidak ada isoteri.toml di direktori ini -- jalankan \"isoteri init\" dulu.".to_string());
    }

    let sumber = if argv.get(1).map(|s| s.as_str()) == Some("--git") {
        let url = argv.get(2).ok_or(PAKAI)?.clone();
        let (mut tag, mut rev) = (None, None);
        let mut i = 3;
        while i < argv.len() {
            match argv[i].as_str() {
                "--tag" => { tag = Some(argv.get(i + 1).ok_or("--tag butuh nilai, mis. --tag v1.0.0")?.clone()); i += 2; }
                "--rev" => { rev = Some(argv.get(i + 1).ok_or("--rev butuh nilai, mis. --rev abcdef1")?.clone()); i += 2; }
                lain => return Err(format!("Opsi tidak dikenal: \"{}\".\n{}", lain, PAKAI)),
            }
        }
        if tag.is_some() && rev.is_some() {
            return Err("Pilih salah satu: --tag ATAU --rev, tidak boleh keduanya.".to_string());
        }
        if tag.is_none() && rev.is_none() {
            return Err(format!("isoteri tambah --git butuh --tag (rilis) atau --rev (commit hash).\n{}", PAKAI));
        }
        eprintln!("Mengambil \"{}\" dari {} ...", nama_paket, url);
        let target = isoteri::resolusi_paket_git(&url, tag.as_deref(), rev.as_deref())?;
        let lib_target = target.join("src").join("lib.iso");
        if !lib_target.exists() {
            eprintln!("Peringatan: \"{}\" tidak ditemukan -- pastikan repo ini punya src/lib.iso sebelum di-'muat'.", lib_target.display());
        }
        eprintln!("Tersimpan di cache: {}", target.display());
        isoteri::SumberDependensi::Git { url, tag, rev }
    } else {
        let path_paket = argv.get(1).ok_or(PAKAI)?.clone();
        let lib_target = Path::new(&path_paket).join("src").join("lib.iso");
        if !lib_target.exists() {
            eprintln!("Peringatan: \"{}\" tidak ditemukan -- pastikan paket \"{}\" punya src/lib.iso sebelum di-'muat'.", lib_target.display(), nama_paket);
        }
        isoteri::SumberDependensi::Lokal(path_paket)
    };

    let mut manifest = isoteri::baca_manifest(Path::new("isoteri.toml"))?;
    manifest.dependensi.insert(nama_paket.clone(), sumber);

    let mut keluar = format!("nama = \"{}\"\nversi = \"{}\"\n\n[dependensi]\n", manifest.nama, manifest.versi);
    let mut nama_dep: Vec<&String> = manifest.dependensi.keys().collect();
    nama_dep.sort();
    for n in nama_dep {
        let baris = match &manifest.dependensi[n] {
            isoteri::SumberDependensi::Lokal(p) => format!("{} = {{ path = \"{}\" }}\n", n, p),
            isoteri::SumberDependensi::Git { url, tag: Some(t), .. } => format!("{} = {{ git = \"{}\", tag = \"{}\" }}\n", n, url, t),
            isoteri::SumberDependensi::Git { url, rev: Some(r), .. } => format!("{} = {{ git = \"{}\", rev = \"{}\" }}\n", n, url, r),
            isoteri::SumberDependensi::Git { url, .. } => format!("{} = {{ git = \"{}\" }}\n", n, url), // tak tercapai (tag/rev wajib salah satu)
        };
        keluar.push_str(&baris);
    }
    fs::write("isoteri.toml", keluar).map_err(|e| format!("Gagal menulis isoteri.toml: {}", e))?;
    eprintln!("Ditambahkan: {} (dimuat lewat: muat \"{}\")", nama_paket, nama_paket);
    Ok(())
}

/// Subcommand `isoteri uji [direktori]` -- jalankan tiap `.iso` di `direktori` (default
/// "tes/"), satu file = satu kasus uji. Konvensi: pakai `kalau (bukan kondisi) { gagal_uji("pesan") }`
/// -- `gagal_uji()` menghentikan eksekusi dengan error, yang di sini ditangkap sebagai GAGAL.
/// Sengaja MINIMAL (bukan framework assertion penuh) -- lihat docs/FILOSOFI.md Milestone C.
fn mode_uji(argv: &[String]) -> i32 {
    let dir = argv.first().cloned().unwrap_or_else(|| "tes".to_string());
    let dir_path = Path::new(&dir);
    if !dir_path.is_dir() {
        eprintln!("Direktori uji \"{}\" tidak ditemukan.", dir);
        return 1;
    }
    let mut berkas: Vec<_> = match fs::read_dir(dir_path) {
        Ok(rd) => rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().map(|e| e == "iso").unwrap_or(false)).collect(),
        Err(e) => { eprintln!("Gagal membaca direktori \"{}\": {}", dir, e); return 1; }
    };
    berkas.sort();
    if berkas.is_empty() {
        eprintln!("Tidak ada berkas .iso di \"{}\".", dir);
        return 0;
    }

    let mut lulus = 0;
    let mut gagal = 0;
    for p in &berkas {
        let nama = p.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
        print!("uji {} ... ", nama);
        match isoteri::jalankan_berkas(&p.to_string_lossy()) {
            Ok(()) => { println!("LULUS"); lulus += 1; }
            Err(e) => { println!("GAGAL\n  {}", e); gagal += 1; }
        }
    }
    println!("---\n{} lulus, {} gagal, dari {} kasus uji.", lulus, gagal, berkas.len());
    if gagal > 0 { 1 } else { 0 }
}

/// Subcommand `isoteri ekspor-web program.iso -o program.isoweb.json` -- kompilasi
/// program (+ semua 'muat'-nya) jadi SATU bundel bytecode JSON yang bisa dijalankan
/// di browser lewat runtime/web/isoteri-vm.js, tanpa perlu target wasm32-unknown-unknown
/// (lihat docs/FILOSOFI.md bagian Fase 3 untuk alasan pendekatan ini).
fn mode_ekspor_web(argv: &[String]) -> Result<(), String> {
    let mut path_masukan: Option<String> = None;
    let mut path_keluaran: Option<String> = None;
    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "-o" | "--keluaran" => {
                i += 1;
                path_keluaran = Some(argv.get(i).ok_or("butuh nama berkas setelah -o")?.clone());
            }
            lain => path_masukan = Some(lain.to_string()),
        }
        i += 1;
    }
    let path_masukan = path_masukan.ok_or("pakai: isoteri ekspor-web program.iso -o program.isoweb.json")?;
    let nama_program = Path::new(&path_masukan)
        .file_stem().and_then(|s| s.to_str()).unwrap_or("program").to_string();
    let path_keluaran = path_keluaran.unwrap_or_else(|| format!("{}.isoweb.json", nama_program));

    eprintln!("Mengumpulkan & mengompilasi bytecode untuk web...");
    let json = isoteri::ekspor_json_dari_berkas(&path_masukan)?;
    fs::write(&path_keluaran, json).map_err(|e| format!("Gagal menulis \"{}\": {}", path_keluaran, e))?;
    eprintln!("Selesai. Bundel bytecode: {}", path_keluaran);
    eprintln!("Jalankan di browser via runtime/web/ (lihat runtime/web/README.md), atau di Node.js:");
    eprintln!("  node runtime/web/jalankan-node.js {}", path_keluaran);
    Ok(())
}

/// Subcommand `isoteri bangun program.iso -o keluaran` -- kompilasi AOT: bundel program .iso
/// (+ semua yang di-'muat'-nya) jadi SATU executable native mandiri, yang bisa dijalankan
/// langsung tanpa perlu `isoteri` atau berkas .iso terpisah lagi. Caranya: kumpulkan semua
/// sumber jadi satu teks gabungan (lewat isoteri::kumpulkan_sumber_gabungan), tempel sebagai
/// string literal ke sebuah crate Rust kecil yang cuma manggil isoteri::jalankan_sumber(),
/// lalu `cargo build --release` crate itu.
fn mode_bangun(argv: &[String]) -> Result<(), String> {
    let mut path_masukan: Option<String> = None;
    let mut path_keluaran: Option<String> = None;
    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "-o" | "--keluaran" => {
                i += 1;
                path_keluaran = Some(argv.get(i).ok_or("butuh nama berkas setelah -o")?.clone());
            }
            lain => path_masukan = Some(lain.to_string()),
        }
        i += 1;
    }
    let path_masukan = path_masukan.ok_or("pakai: isoteri bangun program.iso -o nama_keluaran")?;
    let nama_program = Path::new(&path_masukan)
        .file_stem().and_then(|s| s.to_str()).unwrap_or("program_isoteri").to_string();
    let path_keluaran = path_keluaran.unwrap_or_else(|| nama_program.clone());

    eprintln!("Mengumpulkan sumber (mengikuti semua 'muat')...");
    let sumber_gabungan = isoteri::kumpulkan_sumber_gabungan(&path_masukan)?;

    eprintln!("Memvalidasi program sebelum dibangun...");
    isoteri::periksa_sumber(&sumber_gabungan)?;

    // Direktori kerja SENGAJA stabil (bukan per-PID) supaya dependency (cranelift, ureq, dst.)
    // yang sudah dikompilasi di panggilan sebelumnya bisa dipakai ulang oleh cargo secara
    // incremental -- tanpa ini, tiap `bangun` butuh ~3-4 menit kompilasi ulang dari nol.
    let dir_kerja = std::env::temp_dir().join("isoteri-bangun-cache");
    fs::create_dir_all(dir_kerja.join("src")).map_err(|e| e.to_string())?;

    let dir_isoteri = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .and_then(|p| p.parent().map(|p| p.to_path_buf())) // keluar dari target/release
        .and_then(|p| p.parent().map(|p| p.to_path_buf())) // keluar dari target
        .unwrap_or_else(|| Path::new(".").to_path_buf());

    let cargo_toml = format!(
        "[package]\nname = \"{nama}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"{nama}\"\npath = \"src/main.rs\"\n\n[dependencies]\nisoteri = {{ path = {path:?} }}\n",
        nama = nama_program,
        path = dir_isoteri.display().to_string(),
    );
    fs::write(dir_kerja.join("Cargo.toml"), cargo_toml).map_err(|e| e.to_string())?;

    let main_rs = format!(
        "// Dihasilkan otomatis oleh `isoteri bangun` -- jangan diedit manual.\n// Jalan lewat pipeline IR linear (docs/IR.md) -- bytecode & JIT-nya generate dari\n// representasi IR yang sama, bukan menelusuri AST langsung seperti versi lama.\nconst SUMBER_PROGRAM: &str = {sumber:?};\n\nfn main() {{\n    if let Err(e) = isoteri::jalankan_sumber_via_ir(SUMBER_PROGRAM) {{\n        eprintln!(\"{{}}\", e);\n        std::process::exit(1);\n    }}\n}}\n",
        sumber = sumber_gabungan,
    );
    fs::write(dir_kerja.join("src/main.rs"), main_rs).map_err(|e| e.to_string())?;

    // Salin Cargo.lock dari proyek isoteri sendiri kalau ada -- di environment dengan rustc
    // lama, ini sudah berisi pin versi dependency yang terbukti kompatibel (lihat
    // docs/INSTALASI.md), jadi crate bundel gak perlu nemuin ulang masalah yang sama.
    let lock_asal = dir_isoteri.join("Cargo.lock");
    if lock_asal.exists() {
        let _ = fs::copy(&lock_asal, dir_kerja.join("Cargo.lock"));
    }

    eprintln!("Mengompilasi executable native (cargo build --release)...");
    let status = Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(&dir_kerja)
        .status()
        .map_err(|e| format!("Gagal menjalankan cargo: {}", e))?;
    if !status.success() {
        return Err("cargo build gagal -- lihat pesan error di atas.".to_string());
    }

    let hasil_bin = dir_kerja.join("target/release").join(&nama_program);
    fs::copy(&hasil_bin, &path_keluaran).map_err(|e| format!("Gagal menyalin hasil build ke \"{}\": {}", path_keluaran, e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(&path_keluaran) {
            let mut perm = meta.permissions();
            perm.set_mode(perm.mode() | 0o111);
            let _ = fs::set_permissions(&path_keluaran, perm);
        }
    }

    eprintln!("Selesai. Executable: {}", path_keluaran);
    Ok(())
}

/// Subcommand `isoteri format berkas1.iso [berkas2.iso ...] [--cek]` -- format ulang di
/// tempat (menimpa berkasnya), atau (dengan --cek) cuma memeriksa apakah SUDAH terformat
/// tanpa menulis apa pun -- cocok buat CI (exit code nonzero kalau ada yang belum rapi).
/// "Formatter adalah sumber kebenaran gaya penulisan" (docs/FILOSOFI.md) -- lihat
/// isoteri::format_sumber buat penjelasan lengkap pendekatan & keterbatasan v1.
fn mode_format(argv: &[String]) -> i32 {
    let cek_saja = argv.iter().any(|a| a == "--cek" || a == "--check");
    let berkas: Vec<&String> = argv.iter().filter(|a| a.as_str() != "--cek" && a.as_str() != "--check").collect();
    if berkas.is_empty() {
        eprintln!("pakai: isoteri format berkas1.iso [berkas2.iso ...] [--cek]");
        return 1;
    }

    let mut ada_masalah = false;
    for p in berkas {
        let sumber_asli = match fs::read_to_string(p) {
            Ok(s) => s,
            Err(e) => { eprintln!("Gagal membaca \"{}\": {}", p, e); ada_masalah = true; continue; }
        };
        let hasil = match isoteri::format_sumber(&sumber_asli) {
            Ok(h) => h,
            Err(e) => { eprintln!("{}: {}", p, e); ada_masalah = true; continue; }
        };
        if hasil == sumber_asli {
            eprintln!("{}: sudah rapi", p);
            continue;
        }
        if cek_saja {
            eprintln!("{}: BELUM terformat (jalankan \"isoteri format {}\" buat merapikan)", p, p);
            ada_masalah = true;
        } else {
            if let Err(e) = fs::write(p, &hasil) {
                eprintln!("Gagal menulis \"{}\": {}", p, e);
                ada_masalah = true;
                continue;
            }
            eprintln!("{}: dirapikan", p);
        }
    }
    if ada_masalah { 1 } else { 0 }
}
