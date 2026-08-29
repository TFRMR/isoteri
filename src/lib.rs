use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::rc::Rc;

// =====================================================================
// 1. TOKEN & LEXER
// =====================================================================

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Ingat, Kalau, Lainnya, Ulang, Tampilkan, Fungsi, Kembalikan, Muat, Sebagai,
    Dan, Atau, Benar, Salah, Setiap, Dari, Selaras, Coba, Tangkap, Bentuk,
    Putus, Lanjut,

    Identifikator(String),
    Teks(String),
    Angka(i64),
    AngkaDesimal(f64),

    SamaDengan, SamaDenganDua, TidakSama, Seru,
    LebihBesar, LebihBesarSamaDengan, LebihKecil, LebihKecilSamaDengan,
    Tambah, Kurang, Kali, Bagi, Persen,
    /// Compound assignment (+= -= *= /=) -- gula sintaksis MURNI di parser, langsung didesugar
    /// jadi 'nama = nama <op> nilai' (lihat parse_stmt). Tidak ada varian Stmt/CStmt/Instr baru
    /// buat ini sama sekali, jadi otomatis jalan di semua jalur eksekusi termasuk web export.
    TambahSama, KurangSama, KaliSama, BagiSama,
    /// ++/-- -- sama, gula sintaksis murni, didesugar jadi 'nama = nama + 1' / 'nama = nama - 1'.
    /// Cuma didukung sebagai STATEMENT ('i++' baris sendiri), bukan ekspresi ('x = i++' tidak
    /// didukung) -- itu jaga supaya tetap sederhana dan gak nambah kerumitan urutan evaluasi.
    PlusPlus, MinusMinus,
    Titik, TitikDua,
    KurungBuka, KurungTutup, KurawalBuka, KurawalTutup,
    KurungSikuBuka, KurungSikuTutup, Koma,

    Eof,
    /// Dipakai HANYA oleh `tokenize_dengan_komentar` (lihat bagian "11. FORMATTER") -- Parser
    /// TIDAK PERNAH melihat token ini (difilter sebelum dikirim ke Parser::new), jadi
    /// menambah varian ini aman buat kode compiler yang sudah ada.
    Komentar(String),
}

pub struct Lexer { input: Vec<char>, posisi: usize, baris: usize, pertahankan_komentar: bool }

impl Lexer {
    pub fn new(input: &str) -> Self { Lexer { input: input.chars().collect(), posisi: 0, baris: 1, pertahankan_komentar: false } }

    /// Sama seperti `tokenize()`, TAPI baris `catatan: ...` diemit sebagai `Token::Komentar`,
    /// bukan dibuang -- dipakai HANYA oleh formatter (bagian "11"), supaya bisa tahu di baris
    /// berapa komentar aslinya berada lalu menempelkannya kembali ke output yang diformat ulang.
    /// Path kompilasi normal (`tokenize()`) SAMA SEKALI TIDAK BERUBAH -- nol risiko regresi ke
    /// compiler/VM yang sudah ada.
    pub fn tokenize_dengan_komentar(&mut self) -> Result<Vec<(Token, usize)>, String> {
        self.pertahankan_komentar = true;
        self.tokenize()
    }

    pub fn tokenize(&mut self) -> Result<Vec<(Token, usize)>, String> {
        let mut tokens: Vec<(Token, usize)> = Vec::new();
        macro_rules! push { ($t:expr) => { tokens.push(($t, self.baris)); } }
        while self.posisi < self.input.len() {
            let ch = self.input[self.posisi];
            if ch == '\n' { self.baris += 1; self.posisi += 1; continue; }
            if ch.is_whitespace() { self.posisi += 1; continue; }

            if self.cek_kata_depan("catatan:") {
                let mulai = self.posisi;
                while self.posisi < self.input.len() && self.input[self.posisi] != '\n' { self.posisi += 1; }
                if self.pertahankan_komentar {
                    let teks: String = self.input[mulai..self.posisi].iter().collect();
                    push!(Token::Komentar(teks));
                }
                continue;
            }

            if ch == '"' {
                self.posisi += 1;
                let mut teks = String::new();
                while self.posisi < self.input.len() && self.input[self.posisi] != '"' {
                    if self.input[self.posisi] == '\\' && self.posisi + 1 < self.input.len() {
                        match self.input[self.posisi + 1] {
                            '"' => { teks.push('"'); self.posisi += 2; continue; }
                            'n' => { teks.push('\n'); self.posisi += 2; continue; }
                            '\\' => { teks.push('\\'); self.posisi += 2; continue; }
                            _ => {}
                        }
                    }
                    teks.push(self.input[self.posisi]);
                    self.posisi += 1;
                }
                if self.posisi >= self.input.len() { return Err("Teks tidak ditutup dengan tanda kutip (\")".to_string()); }
                self.posisi += 1;
                push!(Token::Teks(teks));
                continue;
            }

            if ch.is_ascii_digit() {
                let mut angka_str = String::new();
                while self.posisi < self.input.len() && self.input[self.posisi].is_ascii_digit() {
                    angka_str.push(self.input[self.posisi]);
                    self.posisi += 1;
                }
                let mut desimal = false;
                if self.posisi < self.input.len() && self.input[self.posisi] == '.'
                    && self.posisi + 1 < self.input.len() && self.input[self.posisi + 1].is_ascii_digit()
                {
                    desimal = true;
                    angka_str.push('.');
                    self.posisi += 1;
                    while self.posisi < self.input.len() && self.input[self.posisi].is_ascii_digit() {
                        angka_str.push(self.input[self.posisi]);
                        self.posisi += 1;
                    }
                }
                if desimal {
                    let n: f64 = angka_str.parse().map_err(|_| format!("Baris {}: Literal desimal \"{}\" tidak valid.", self.baris, angka_str))?;
                    push!(Token::AngkaDesimal(n));
                } else {
                    // SENGAJA tidak pakai unwrap_or(0): literal yang gagal di-parse (kegedean
                    // buat i64, mis. lebih dari 9223372036854775807) HARUS jadi error kompilasi
                    // yang jelas, bukan diam-diam jadi 0 -- itu bakal jadi bug tersembunyi yang
                    // sangat membingungkan buat pemula (angka yang ditulis salah malah dianggap
                    // valid tapi hasilnya nol).
                    let n: i64 = angka_str.parse().map_err(|_| format!("Baris {}: Literal angka \"{}\" tidak valid atau di luar jangkauan Angka (maksimum {}).", self.baris, angka_str, i64::MAX))?;
                    push!(Token::Angka(n));
                }
                continue;
            }

            if ch.is_alphabetic() || ch == '_' {
                let mut kata = String::new();
                while self.posisi < self.input.len()
                    && (self.input[self.posisi].is_alphanumeric() || self.input[self.posisi] == '_')
                {
                    kata.push(self.input[self.posisi]);
                    self.posisi += 1;
                }
                let token = match kata.to_lowercase().as_str() {
                    "ingat" | "simpan" => Token::Ingat,
                    "jika" | "kalau" => Token::Kalau,
                    "lainnya" => Token::Lainnya,
                    "ulang" => Token::Ulang,
                    "tampilkan" => Token::Tampilkan,
                    "fungsi" => Token::Fungsi,
                    "kembalikan" => Token::Kembalikan,
                    "muat" => Token::Muat,
                    "sebagai" => Token::Sebagai,
                    "dan" => Token::Dan,
                    "atau" => Token::Atau,
                    "benar" => Token::Benar,
                    "salah" => Token::Salah,
                    "setiap" => Token::Setiap,
                    "dari" => Token::Dari,
                    "selaras" => Token::Selaras,
                    "coba" => Token::Coba,
                    "tangkap" => Token::Tangkap,
                    "bentuk" => Token::Bentuk,
                    "putus" | "berhenti" => Token::Putus,
                    "lanjut" | "lanjutkan" => Token::Lanjut,
                    _ => Token::Identifikator(kata),
                };
                push!(token);
                continue;
            }

            match ch {
                '=' => { if self.intip() == '=' { push!(Token::SamaDenganDua); self.posisi += 2; } else { push!(Token::SamaDengan); self.posisi += 1; } }
                '!' => { if self.intip() == '=' { push!(Token::TidakSama); self.posisi += 2; } else { push!(Token::Seru); self.posisi += 1; } }
                '>' => { if self.intip() == '=' { push!(Token::LebihBesarSamaDengan); self.posisi += 2; } else { push!(Token::LebihBesar); self.posisi += 1; } }
                '<' => { if self.intip() == '=' { push!(Token::LebihKecilSamaDengan); self.posisi += 2; } else { push!(Token::LebihKecil); self.posisi += 1; } }
                '+' => {
                    if self.intip() == '=' { push!(Token::TambahSama); self.posisi += 2; }
                    else if self.intip() == '+' { push!(Token::PlusPlus); self.posisi += 2; }
                    else { push!(Token::Tambah); self.posisi += 1; }
                }
                '-' => {
                    if self.intip() == '=' { push!(Token::KurangSama); self.posisi += 2; }
                    else if self.intip() == '-' { push!(Token::MinusMinus); self.posisi += 2; }
                    else { push!(Token::Kurang); self.posisi += 1; }
                }
                '*' => { if self.intip() == '=' { push!(Token::KaliSama); self.posisi += 2; } else { push!(Token::Kali); self.posisi += 1; } }
                '/' => { if self.intip() == '=' { push!(Token::BagiSama); self.posisi += 2; } else { push!(Token::Bagi); self.posisi += 1; } }
                '%' => { push!(Token::Persen); self.posisi += 1; }
                '.' => { push!(Token::Titik); self.posisi += 1; }
                ':' => { push!(Token::TitikDua); self.posisi += 1; }
                '(' => { push!(Token::KurungBuka); self.posisi += 1; }
                ')' => { push!(Token::KurungTutup); self.posisi += 1; }
                '{' => { push!(Token::KurawalBuka); self.posisi += 1; }
                '}' => { push!(Token::KurawalTutup); self.posisi += 1; }
                '[' => { push!(Token::KurungSikuBuka); self.posisi += 1; }
                ']' => { push!(Token::KurungSikuTutup); self.posisi += 1; }
                ',' => { push!(Token::Koma); self.posisi += 1; }
                lain => return Err(format!("Karakter tidak dikenal: '{}' pada baris {}", lain, self.baris)),
            }
        }
        tokens.push((Token::Eof, self.baris));
        Ok(tokens)
    }

    fn intip(&self) -> char { if self.posisi + 1 < self.input.len() { self.input[self.posisi + 1] } else { '\0' } }
    fn cek_kata_depan(&self, awalan: &str) -> bool { self.input[self.posisi..].iter().collect::<String>().starts_with(awalan) }
}

// =====================================================================
// 2. AST MENTAH & PARSER
// =====================================================================

#[derive(Debug, Clone)]
pub enum Expr {
    Angka(i64), Desimal(f64), Teks(String), Bool(bool), Ident(String),
    Binary(Box<Expr>, BinOp, Box<Expr>),
    Panggil(String, Vec<Expr>),
    Daftar(Vec<Expr>),
    Peta(Vec<(String, Expr)>),
    Indeks(Box<Expr>, Box<Expr>),
    Field(Box<Expr>, String),
    /// 'x.y(args)' -- BEDA dari Field (yang cuma baca, tanpa panggil). Resolver memutuskan
    /// artinya sesuai konteks: kalau 'x' adalah identifier alias modul dikenal (dari
    /// 'muat "..." sebagai x'), ini jadi panggilan fungsi modul biasa (dikompilasi ulang jadi
    /// Panggil dengan nama "x.y" yang sudah di-mangle -- lihat tulis_ulang_panggil_alias()).
    /// Kalau bukan, ini jadi "baca field y dari x, lalu panggil NILAINYA sebagai fungsi" (mis.
    /// closure yang disimpan di field bentuk) -- lihat PanggilNilai.
    PanggilMetode(Box<Expr>, String, Vec<Expr>),
    /// Negasi boolean unary ('!ekspr') -- pakai truthiness YANG SAMA seperti kondisi 'kalau'/
    /// 'dan'/'atau' (lihat Value::truthy()), bukan cuma 'ekspr == salah'. Jadi '!5' itu Salah
    /// (5 truthy), '!0' itu Benar, '!""' itu Benar, dst -- konsisten di seluruh bahasa.
    Tidak(Box<Expr>),
    BentukLiteral(String, Vec<(String, Expr)>),
    /// fungsi(params) { badan } dipakai sebagai EKSPRESI (closure/fungsi anonim) -- beda dari
    /// Stmt::FungsiDef yang punya nama & cuma boleh di level atas. Closure ini boleh nangkep
    /// variabel dari scope pembungkusnya (lihat fn variabel_bebas_stmt & resolve_fungsi_umum).
    FungsiLiteral(Vec<(String, Option<String>)>, Vec<(usize, Stmt)>),
}

/// Satu langkah dalam rantai lvalue assignment SETELAH nama variabel awal --
/// mis. 'daftar[0]' -> [Indeks(Angka 0)], 'matriks[0][1]' -> [Indeks(0), Indeks(1)],
/// 'objek.daftar[0]' -> [Field("daftar"), Indeks(0)]. Dipakai KHUSUS buat sisi kiri
/// assignment (lihat Stmt::UbahJalur) -- beda dari Expr::Indeks/Expr::Field yang dipakai buat
/// sisi kanan/baca biasa, meskipun secara bentuk mirip (memang sengaja, biar gampang saling
/// dikonversi lewat Parser::bangun_expr_dari_jalur).
#[derive(Debug, Clone)]
pub enum Jalur { Field(String), Indeks(Expr) }

#[derive(Debug, Clone, Copy)]
pub enum BinOp { Tambah, Kurang, Kali, Bagi, Modulo, SamaDengan, TidakSama, LebihBesar, LebihBesarSama, LebihKecil, LebihKecilSama, Dan, Atau }

#[derive(Debug, Clone)]
pub enum Stmt {
    Ingat(String, Option<String>, Expr),
    Ubah(String, Expr),
    UbahField(String, Vec<String>, Expr),
    BentukDef(String, Vec<(String, Option<String>)>),
    /// muat "path/relatif.iso" [sebagai alias] -- diekspansi (diganti isi file itu) SEBELUM
    /// resolver jalan, lihat fn ekspansi_muat(). Cuma boleh muncul di level atas program.
    /// Tanpa 'sebagai': isi file di-inline flat ke namespace global seperti biasa (perilaku
    /// lama, backward-compatible penuh). Dengan 'sebagai alias': nama fungsi top-level modul
    /// itu di-mangle jadi "alias.nama" (nggak numplek ke namespace global), diakses lewat
    /// 'alias.nama(...)' -- lihat ekspansi_muat_beralias().
    Muat(String, Option<String>),
    Tampilkan(Expr),
    Kalau(Expr, Vec<(usize, Stmt)>, Option<Vec<(usize, Stmt)>>),
    Ulang(Expr, Vec<(usize, Stmt)>),
    UlangSetiap(String, Expr, Vec<(usize, Stmt)>),
    UlangSelaras(String, Expr, Vec<(usize, Stmt)>),
    FungsiDef(String, Vec<(String, Option<String>)>, Vec<(usize, Stmt)>),
    Kembalikan(Expr),
    EkspresiStmt(Expr),
    Coba(Vec<(usize, Stmt)>, String, Vec<(usize, Stmt)>),
    /// 'daftar[0] = x', 'peta["k"] = x', 'matriks[0][1] = x', 'objek.daftar[0] = x', dst. --
    /// rantai Jalur (Field/Indeks campur, boleh berapa level) di atas satu variabel dasar.
    /// Dipisah dari UbahField (yang cuma field murni) supaya UbahField yang sudah lama
    /// stabil TIDAK disentuh sama sekali -- lihat Parser::parse_stmt, rute ini cuma dipakai
    /// kalau rantainya mengandung MINIMAL satu '[...]'.
    UbahJalur(String, Vec<Jalur>, Expr),
    /// 'putus' -- keluar paksa dari loop terdekat yang membungkusnya (ulang/ulang setiap).
    /// Divalidasi resolver: error kompilasi kalau dipakai di luar loop.
    Putus,
    /// 'lanjut' -- lompat ke iterasi berikutnya loop terdekat yang membungkusnya.
    Lanjut,
}

pub struct Parser { tokens: Vec<Token>, baris_token: Vec<usize>, posisi: usize, no_literal: bool }

impl Parser {
    pub fn new(token_berbaris: Vec<(Token, usize)>) -> Self {
        let tokens = token_berbaris.iter().map(|(t, _)| t.clone()).collect();
        let baris_token = token_berbaris.iter().map(|(_, b)| *b).collect();
        Parser { tokens, baris_token, posisi: 0, no_literal: false }
    }
    fn sekarang(&self) -> &Token { &self.tokens[self.posisi] }
    fn baris_sekarang(&self) -> usize { self.baris_token[self.posisi] }
    /// Begitu masuk konteks berkurung (isi "(...)", "[...]", argumen panggilan, dst.),
    /// ambiguitas "identifier diikuti '{'" hilang lagi -- jadi flag no_literal dilepas
    /// sementara, lalu dikembalikan ke nilai sebelumnya setelah keluar dari sini.
    fn dalam_kurung<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T, String>) -> Result<T, String> {
        let sebelum = std::mem::replace(&mut self.no_literal, false);
        let hasil = f(self);
        self.no_literal = sebelum;
        hasil
    }
    fn maju(&mut self) -> Token {
        let t = self.tokens[self.posisi].clone();
        if self.posisi < self.tokens.len() - 1 { self.posisi += 1; }
        t
    }
    fn harap(&mut self, expected: &Token) -> Result<(), String> {
        if std::mem::discriminant(self.sekarang()) == std::mem::discriminant(expected) { self.maju(); Ok(()) }
        else { Err(format!("Baris {}: Diharapkan {:?}, tapi ditemukan {:?}", self.baris_sekarang(), expected, self.sekarang())) }
    }

    pub fn parse_program(&mut self) -> Result<Vec<(usize, Stmt)>, String> {
        let mut stmts = Vec::new();
        while *self.sekarang() != Token::Eof {
            let baris = self.baris_sekarang();
            stmts.push((baris, self.parse_stmt()?));
        }
        Ok(stmts)
    }

    fn parse_block(&mut self) -> Result<Vec<(usize, Stmt)>, String> {
        self.harap(&Token::KurawalBuka)?;
        let mut stmts = Vec::new();
        while *self.sekarang() != Token::KurawalTutup && *self.sekarang() != Token::Eof {
            let baris = self.baris_sekarang();
            stmts.push((baris, self.parse_stmt()?));
        }
        self.harap(&Token::KurawalTutup)?;
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        match self.sekarang().clone() {
            Token::Ingat => {
                self.maju();
                let nama = self.harap_identifier()?;
                let tipe = if *self.sekarang() == Token::TitikDua { self.maju(); Some(self.harap_identifier()?) } else { None };
                self.harap(&Token::SamaDengan)?;
                Ok(Stmt::Ingat(nama, tipe, self.parse_expr()?))
            }
            Token::Tampilkan => { self.maju(); Ok(Stmt::Tampilkan(self.parse_expr()?)) }
            Token::Kalau => {
                self.maju();
                self.harap(&Token::KurungBuka)?;
                let cond = self.parse_expr()?;
                self.harap(&Token::KurungTutup)?;
                let then_block = self.parse_block()?;
                let else_block = if *self.sekarang() == Token::Lainnya {
                    self.maju();
                    if *self.sekarang() == Token::Kalau {
                        // 'lainnya kalau (...) {...}' -- gula sintaksis murni: rantai ini
                        // diparse ULANG lewat parse_stmt() (rekursif ke arm Token::Kalau di
                        // atas), lalu dibungkus jadi blok satu-statement. Tidak ada varian AST
                        // baru buat else-if -- CStmt/compiler/VM/JIT sama sekali tidak berubah,
                        // karena secara struktur ini tetap 'Kalau' bersarang di dalam 'lainnya'.
                        let baris = self.baris_sekarang();
                        Some(vec![(baris, self.parse_stmt()?)])
                    } else {
                        Some(self.parse_block()?)
                    }
                } else { None };
                Ok(Stmt::Kalau(cond, then_block, else_block))
            }
            Token::Ulang => {
                self.maju();
                if *self.sekarang() == Token::Selaras {
                    self.maju();
                    self.harap(&Token::Setiap)?;
                    let var = self.harap_identifier()?;
                    self.harap(&Token::Dari)?;
                    let sebelum = std::mem::replace(&mut self.no_literal, true);
                    let daftar_expr = self.parse_expr()?;
                    self.no_literal = sebelum;
                    let body = self.parse_block()?;
                    Ok(Stmt::UlangSelaras(var, daftar_expr, body))
                } else if *self.sekarang() == Token::Setiap {
                    self.maju();
                    let var = self.harap_identifier()?;
                    self.harap(&Token::Dari)?;
                    let sebelum = std::mem::replace(&mut self.no_literal, true);
                    let daftar_expr = self.parse_expr()?;
                    self.no_literal = sebelum;
                    let body = self.parse_block()?;
                    Ok(Stmt::UlangSetiap(var, daftar_expr, body))
                } else {
                    self.harap(&Token::KurungBuka)?;
                    let cond = self.parse_expr()?;
                    self.harap(&Token::KurungTutup)?;
                    Ok(Stmt::Ulang(cond, self.parse_block()?))
                }
            }
            Token::Fungsi => {
                self.maju();
                let nama = self.harap_identifier()?;
                self.harap(&Token::KurungBuka)?;
                let params = self.parse_daftar_parameter()?;
                self.harap(&Token::KurungTutup)?;
                Ok(Stmt::FungsiDef(nama, params, self.parse_block()?))
            }
            Token::Muat => {
                self.maju();
                match self.sekarang().clone() {
                    Token::Teks(path) => {
                        self.maju();
                        let alias = if *self.sekarang() == Token::Sebagai {
                            self.maju();
                            Some(self.harap_identifier()?)
                        } else { None };
                        Ok(Stmt::Muat(path, alias))
                    }
                    lain => Err(format!("Baris {}: Diharapkan nama berkas (Teks) setelah 'muat', ditemukan {:?}", self.baris_sekarang(), lain)),
                }
            }
            Token::Bentuk => {
                self.maju();
                let nama = self.harap_identifier()?;
                self.harap(&Token::KurawalBuka)?;
                let mut fields: Vec<(String, Option<String>)> = Vec::new();
                if *self.sekarang() != Token::KurawalTutup {
                    loop {
                        let fnama = self.harap_identifier()?;
                        let ftipe = if *self.sekarang() == Token::TitikDua { self.maju(); Some(self.harap_identifier()?) } else { None };
                        fields.push((fnama, ftipe));
                        if *self.sekarang() == Token::Koma { self.maju(); } else { break; }
                    }
                }
                self.harap(&Token::KurawalTutup)?;
                Ok(Stmt::BentukDef(nama, fields))
            }
            Token::Kembalikan => { self.maju(); Ok(Stmt::Kembalikan(self.parse_expr()?)) }
            Token::Putus => { self.maju(); Ok(Stmt::Putus) }
            Token::Lanjut => { self.maju(); Ok(Stmt::Lanjut) }
            Token::Coba => {
                self.maju();
                let badan_coba = self.parse_block()?;
                self.harap(&Token::Tangkap)?;
                let nama_var = self.harap_identifier()?;
                let badan_tangkap = self.parse_block()?;
                Ok(Stmt::Coba(badan_coba, nama_var, badan_tangkap))
            }
            Token::Identifikator(nama) => {
                if self.tokens.get(self.posisi + 1) == Some(&Token::SamaDengan) {
                    self.maju(); self.maju();
                    Ok(Stmt::Ubah(nama, self.parse_expr()?))
                } else if let Some(op) = Self::op_compound(self.tokens.get(self.posisi + 1)) {
                    // 'nama += nilai' dst. -- desugar MURNI: 'nama = nama <op> (nilai)'. Tanda
                    // kurung di sekitar rhs jaga presedensi (mis. 'x += 1 + 2' harus jadi
                    // 'x = x + (1 + 2)', bukan '(x + 1) + 2' -- meskipun kebetulan sama untuk +,
                    // ini penting buat '/=' dan '-=': 'x -= 1 + 2' harus 'x - (1+2)' bukan '(x-1)+2'.
                    self.maju(); self.maju();
                    let rhs = self.parse_expr()?;
                    Ok(Stmt::Ubah(nama.clone(), Expr::Binary(Box::new(Expr::Ident(nama)), op, Box::new(rhs))))
                } else if self.tokens.get(self.posisi + 1) == Some(&Token::PlusPlus) || self.tokens.get(self.posisi + 1) == Some(&Token::MinusMinus) {
                    // 'nama++' / 'nama--' -- HANYA sebagai statement baris sendiri, desugar jadi
                    // 'nama = nama + 1' / 'nama = nama - 1'. Tidak didukung sebagai ekspresi
                    // (mis. 'x = i++' TIDAK bisa) -- disengaja, jaga urutan evaluasi tetap simpel.
                    let op = if self.tokens.get(self.posisi + 1) == Some(&Token::PlusPlus) { BinOp::Tambah } else { BinOp::Kurang };
                    self.maju(); self.maju();
                    Ok(Stmt::Ubah(nama.clone(), Expr::Binary(Box::new(Expr::Ident(nama)), op, Box::new(Expr::Angka(1)))))
                } else {
                    // Intip ke depan TANPA mengonsumsi: rantai '.field'/'[expr]' campur, berapa
                    // pun panjangnya, berujung di mana? (Skip isi '[...]' pakai penghitung
                    // kedalaman kurung siku, karena index-nya sendiri boleh ekspresi apa pun
                    // termasuk '[...]' bersarang, mis. 'matriks[baris[0]]'.)
                    let akhir_rantai = self.intip_akhir_rantai_jalur(self.posisi + 1);
                    let compound_di_akhir = Self::op_compound(self.tokens.get(akhir_rantai));
                    let assign_di_akhir = self.tokens.get(akhir_rantai) == Some(&Token::SamaDengan);
                    if akhir_rantai > self.posisi + 1 && (assign_di_akhir || compound_di_akhir.is_some()) {
                        self.maju(); // nama
                        let jalur = self.parse_rantai_jalur()?;
                        let nilai = if let Some(op) = compound_di_akhir {
                            self.maju(); // token compound (+= dst.)
                            let rhs = self.parse_expr()?;
                            let nilai_lama = Self::bangun_expr_dari_jalur(Expr::Ident(nama.clone()), &jalur);
                            Expr::Binary(Box::new(nilai_lama), op, Box::new(rhs))
                        } else {
                            self.maju(); // '='
                            self.parse_expr()?
                        };
                        Ok(Self::buat_ubah_jalur(nama, jalur, nilai))
                    } else {
                        Ok(Stmt::EkspresiStmt(self.parse_expr()?))
                    }
                }
            }
            lain => Err(format!("Pernyataan tidak dikenal, ditemukan token: {:?}", lain)),
        }
    }

    /// Helper buat parse_stmt: kalau token ini salah satu compound-assignment (+= -= *= /=),
    /// balikin BinOp yang sesuai buat didesugar. Bukan bagian dari grammar ekspresi biasa
    /// (compound assignment CUMA sah di posisi statement, persis kayak '=' polos).
    fn op_compound(t: Option<&Token>) -> Option<BinOp> {
        match t {
            Some(Token::TambahSama) => Some(BinOp::Tambah),
            Some(Token::KurangSama) => Some(BinOp::Kurang),
            Some(Token::KaliSama) => Some(BinOp::Kali),
            Some(Token::BagiSama) => Some(BinOp::Bagi),
            _ => None,
        }
    }

    /// Dari posisi token SETELAH nama variabel awal, intip (TANPA konsumsi) sejauh mana rantai
    /// '.field'/'[expr]' berlanjut, balikin indeks token TEPAT SESUDAHNYA. Kalau hasilnya sama
    /// dengan `mulai`, artinya bukan rantai sama sekali (langsung token lain, mis. operator
    /// biner) -- itu ditangani caller sebagai fallback ke ekspresi biasa.
    fn intip_akhir_rantai_jalur(&self, mulai: usize) -> usize {
        let mut la = mulai;
        loop {
            match self.tokens.get(la) {
                Some(Token::Titik) => match self.tokens.get(la + 1) {
                    Some(Token::Identifikator(_)) => { la += 2; }
                    _ => break,
                },
                Some(Token::KurungSikuBuka) => {
                    let mut depth = 1;
                    la += 1;
                    while depth > 0 {
                        match self.tokens.get(la) {
                            Some(Token::KurungSikuBuka) => { depth += 1; la += 1; }
                            Some(Token::KurungSikuTutup) => { depth -= 1; la += 1; }
                            Some(Token::Eof) | None => return la, // malformed -- biar error jelas nanti muncul dari parser ekspresi biasa
                            _ => la += 1,
                        }
                    }
                }
                _ => break,
            }
        }
        la
    }

    /// Parse rantai '.field'/'[expr]' (identifier awal SUDAH dikonsumsi pemanggil) jadi
    /// Vec<Jalur> -- versi "tulis" dari loop postfix yang sama di parse_unary (yang membangun
    /// Expr buat "baca"). Presedensi/urutan levelnya identik, cuma bentuk hasilnya beda.
    fn parse_rantai_jalur(&mut self) -> Result<Vec<Jalur>, String> {
        let mut jalur = Vec::new();
        loop {
            if *self.sekarang() == Token::KurungSikuBuka {
                self.maju();
                let idx = self.dalam_kurung(|p| p.parse_expr())?;
                self.harap(&Token::KurungSikuTutup)?;
                jalur.push(Jalur::Indeks(idx));
            } else if *self.sekarang() == Token::Titik {
                self.maju();
                let f = self.harap_identifier()?;
                jalur.push(Jalur::Field(f));
            } else {
                break;
            }
        }
        Ok(jalur)
    }

    /// Bangun balik Expr baca-biasa dari (basis, jalur) -- dipakai buat menyusun "nilai lama"
    /// pas compound-assignment lewat jalur (mis. 'daftar[0] += 1' butuh baca 'daftar[0]' dulu).
    fn bangun_expr_dari_jalur(basis: Expr, jalur: &[Jalur]) -> Expr {
        let mut e = basis;
        for j in jalur {
            e = match j {
                Jalur::Field(f) => Expr::Field(Box::new(e), f.clone()),
                Jalur::Indeks(idx) => Expr::Indeks(Box::new(e), Box::new(idx.clone())),
            };
        }
        e
    }

    /// Rute Stmt yang tepat buat satu assignment jalur -- kalau semua levelnya Field murni
    /// (tanpa Indeks sama sekali), tetap pakai Stmt::UbahField LAMA (kode itu sudah stabil
    /// lama, tidak disentuh sama sekali oleh perubahan ini). Cuma kalau ADA minimal satu
    /// level Indeks, baru dirutekan ke Stmt::UbahJalur (mekanisme baru).
    fn buat_ubah_jalur(nama: String, jalur: Vec<Jalur>, nilai: Expr) -> Stmt {
        if jalur.iter().all(|j| matches!(j, Jalur::Field(_))) {
            let fields: Vec<String> = jalur.into_iter().map(|j| match j { Jalur::Field(f) => f, _ => unreachable!() }).collect();
            Stmt::UbahField(nama, fields, nilai)
        } else {
            Stmt::UbahJalur(nama, jalur, nilai)
        }
    }

    fn harap_identifier(&mut self) -> Result<String, String> {
        match self.maju() { Token::Identifikator(s) => Ok(s), lain => Err(format!("Diharapkan nama identifier, tapi ditemukan {:?}", lain)) }
    }

    /// Parsing daftar "nama: Tipe, nama2: Tipe2" di dalam kurung parameter fungsi/closure --
    /// diasumsikan '(' sudah dikonsumsi pemanggil, dan berhenti tepat sebelum ')'.
    fn parse_daftar_parameter(&mut self) -> Result<Vec<(String, Option<String>)>, String> {
        let mut params: Vec<(String, Option<String>)> = Vec::new();
        if *self.sekarang() != Token::KurungTutup {
            loop {
                let pnama = self.harap_identifier()?;
                let ptipe = if *self.sekarang() == Token::TitikDua { self.maju(); Some(self.harap_identifier()?) } else { None };
                params.push((pnama, ptipe));
                if *self.sekarang() == Token::Koma { self.maju(); } else { break; }
            }
        }
        Ok(params)
    }

    fn parse_expr(&mut self) -> Result<Expr, String> { self.parse_or() }
    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut kiri = self.parse_and()?;
        while *self.sekarang() == Token::Atau { self.maju(); kiri = Expr::Binary(Box::new(kiri), BinOp::Atau, Box::new(self.parse_and()?)); }
        Ok(kiri)
    }
    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut kiri = self.parse_equality()?;
        while *self.sekarang() == Token::Dan { self.maju(); kiri = Expr::Binary(Box::new(kiri), BinOp::Dan, Box::new(self.parse_equality()?)); }
        Ok(kiri)
    }
    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut kiri = self.parse_comparison()?;
        loop {
            let op = match self.sekarang() { Token::SamaDenganDua => BinOp::SamaDengan, Token::TidakSama => BinOp::TidakSama, _ => break };
            self.maju();
            kiri = Expr::Binary(Box::new(kiri), op, Box::new(self.parse_comparison()?));
        }
        Ok(kiri)
    }
    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut kiri = self.parse_term()?;
        loop {
            let op = match self.sekarang() {
                Token::LebihBesar => BinOp::LebihBesar, Token::LebihBesarSamaDengan => BinOp::LebihBesarSama,
                Token::LebihKecil => BinOp::LebihKecil, Token::LebihKecilSamaDengan => BinOp::LebihKecilSama,
                _ => break,
            };
            self.maju();
            kiri = Expr::Binary(Box::new(kiri), op, Box::new(self.parse_term()?));
        }
        Ok(kiri)
    }
    fn parse_term(&mut self) -> Result<Expr, String> {
        let mut kiri = self.parse_factor()?;
        loop {
            let op = match self.sekarang() { Token::Tambah => BinOp::Tambah, Token::Kurang => BinOp::Kurang, _ => break };
            self.maju();
            kiri = Expr::Binary(Box::new(kiri), op, Box::new(self.parse_factor()?));
        }
        Ok(kiri)
    }
    fn parse_factor(&mut self) -> Result<Expr, String> {
        let mut kiri = self.parse_unary()?;
        loop {
            let op = match self.sekarang() { Token::Kali => BinOp::Kali, Token::Bagi => BinOp::Bagi, Token::Persen => BinOp::Modulo, _ => break };
            self.maju();
            kiri = Expr::Binary(Box::new(kiri), op, Box::new(self.parse_unary()?));
        }
        Ok(kiri)
    }
    fn parse_unary(&mut self) -> Result<Expr, String> {
        if *self.sekarang() == Token::Kurang {
            self.maju();
            let expr = self.parse_unary()?;
            return Ok(Expr::Binary(Box::new(Expr::Angka(0)), BinOp::Kurang, Box::new(expr)));
        }
        if *self.sekarang() == Token::Seru {
            self.maju();
            let expr = self.parse_unary()?; // rekursif -- '!!x' dan '!(!x)' sah, dua-duanya
            return Ok(Expr::Tidak(Box::new(expr)));
        }
        let mut expr = self.parse_primary()?;
        loop {
            if *self.sekarang() == Token::KurungSikuBuka {
                self.maju();
                let idx = self.dalam_kurung(|p| p.parse_expr())?;
                self.harap(&Token::KurungSikuTutup)?;
                expr = Expr::Indeks(Box::new(expr), Box::new(idx));
            } else if *self.sekarang() == Token::Titik {
                self.maju();
                let field = self.harap_identifier()?;
                if *self.sekarang() == Token::KurungBuka {
                    self.maju();
                    let mut args = Vec::new();
                    if *self.sekarang() != Token::KurungTutup {
                        loop {
                            args.push(self.dalam_kurung(|p| p.parse_expr())?);
                            if *self.sekarang() == Token::Koma { self.maju(); } else { break; }
                        }
                    }
                    self.harap(&Token::KurungTutup)?;
                    expr = Expr::PanggilMetode(Box::new(expr), field, args);
                } else {
                    expr = Expr::Field(Box::new(expr), field);
                }
            } else {
                break;
            }
        }
        Ok(expr)
    }
    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.maju() {
            Token::Angka(n) => Ok(Expr::Angka(n)),
            Token::AngkaDesimal(f) => Ok(Expr::Desimal(f)),
            Token::Teks(s) => Ok(Expr::Teks(s)),
            Token::Benar => Ok(Expr::Bool(true)),
            Token::Salah => Ok(Expr::Bool(false)),
            Token::Fungsi => {
                self.harap(&Token::KurungBuka)?;
                let params = self.dalam_kurung(|p| p.parse_daftar_parameter())?;
                self.harap(&Token::KurungTutup)?;
                Ok(Expr::FungsiLiteral(params, self.parse_block()?))
            }
            Token::Identifikator(nama) => {
                if *self.sekarang() == Token::KurungBuka {
                    self.maju();
                    let mut args = Vec::new();
                    if *self.sekarang() != Token::KurungTutup {
                        loop {
                            args.push(self.dalam_kurung(|p| p.parse_expr())?);
                            if *self.sekarang() == Token::Koma { self.maju(); } else { break; }
                        }
                    }
                    self.harap(&Token::KurungTutup)?;
                    Ok(Expr::Panggil(nama, args))
                } else if !self.no_literal && *self.sekarang() == Token::KurawalBuka {
                    self.maju();
                    let mut entries = Vec::new();
                    if *self.sekarang() != Token::KurawalTutup {
                        loop {
                            let fnama = self.harap_identifier()?;
                            self.harap(&Token::TitikDua)?;
                            entries.push((fnama, self.dalam_kurung(|p| p.parse_expr())?));
                            if *self.sekarang() == Token::Koma { self.maju(); } else { break; }
                        }
                    }
                    self.harap(&Token::KurawalTutup)?;
                    Ok(Expr::BentukLiteral(nama, entries))
                } else { Ok(Expr::Ident(nama)) }
            }
            Token::KurungBuka => { let e = self.dalam_kurung(|p| p.parse_expr())?; self.harap(&Token::KurungTutup)?; Ok(e) }
            Token::KurungSikuBuka => {
                let mut elemen = Vec::new();
                if *self.sekarang() != Token::KurungSikuTutup {
                    loop {
                        elemen.push(self.dalam_kurung(|p| p.parse_expr())?);
                        if *self.sekarang() == Token::Koma { self.maju(); } else { break; }
                    }
                }
                self.harap(&Token::KurungSikuTutup)?;
                Ok(Expr::Daftar(elemen))
            }
            Token::KurawalBuka => {
                let mut entries = Vec::new();
                if *self.sekarang() != Token::KurawalTutup {
                    loop {
                        let kunci = match self.maju() { Token::Teks(s) => s, lain => return Err(format!("Kunci Peta harus berupa Teks, ditemukan {:?}", lain)) };
                        self.harap(&Token::TitikDua)?;
                        entries.push((kunci, self.dalam_kurung(|p| p.parse_expr())?));
                        if *self.sekarang() == Token::Koma { self.maju(); } else { break; }
                    }
                }
                self.harap(&Token::KurawalTutup)?;
                Ok(Expr::Peta(entries))
            }
            lain => Err(format!("Ekspresi tidak valid, ditemukan token: {:?}", lain)),
        }
    }
}

// =====================================================================
// 3. AST TERKOMPILASI (slot-based) & RESOLVER
// =====================================================================

#[derive(Debug, Clone)]
pub enum CExpr {
    Angka(i64), Desimal(f64), Teks(String), Bool(bool),
    Global(usize), Local(usize),
    Binary(Box<CExpr>, BinOp, Box<CExpr>),
    Panggil(String, Vec<CExpr>),
    Daftar(Vec<CExpr>),
    Peta(Vec<(String, CExpr)>),
    Indeks(Box<CExpr>, Box<CExpr>),
    Field(Box<CExpr>, String),
    Tidak(Box<CExpr>),
    /// Field sudah diurutkan & divalidasi lengkap terhadap skema 'bentuk' saat resolve --
    /// jadi saat runtime tinggal dorong nilai sesuai urutan, tanpa perlu cek nama lagi.
    BentukLiteral(String, Vec<(String, CExpr)>),
    /// Literal closure: nama sintetis (terdaftar di fungsi_out via resolve_fungsi_umum, di-index
    /// seperti fungsi biasa saat compile), + ekspresi buat ambil tiap nilai tangkapan SAAT INI
    /// dari scope pembungkus (dievaluasi di tempat literalnya muncul, bukan di badan closure-nya).
    FungsiLiteral(String, Vec<CExpr>),
    /// Panggil NILAI (bukan nama fungsi statis) -- dipakai kalau target panggilan ternyata
    /// sebuah variabel (kemungkinan berisi closure), bukan nama fungsi yang dikenal resolver.
    PanggilNilai(Box<CExpr>, Vec<CExpr>),
    /// Evaluasi ekspresi instans SEKALI, simpan salinannya ke slot sementara (buat dipakai
    /// ekstraksi field-field berikutnya lewat CExpr::Local/Global ke slot yang sama), LALU
    /// langsung ambil satu field darinya. Dipakai buat memanggil fungsi dengan parameter
    /// "flattened" (lihat ekspansi_panggilan_args) ketika argumennya bukan variabel polos
    /// (mis. panggilan fungsi lain atau literal 'bentuk' langsung) -- tanpa ini, ekspresinya
    /// harus dievaluasi ulang per field, yang salah kalau ada efek samping/mahal.
    SimpanLaluField(Box<CExpr>, SlotSasaran, String),
}

#[derive(Debug, Clone, Copy)]
pub enum SlotSasaran { Lokal(usize), Global(usize) }

/// Versi CExpr dari Jalur (AST mentah) -- resolved, indeksnya sudah CExpr (bukan Expr lagi).
#[derive(Debug, Clone)]
pub enum CJalur { Field(String), Indeks(CExpr) }

#[derive(Debug, Clone)]
pub enum CStmt {
    IngatGlobal(usize, CExpr), UbahGlobal(usize, CExpr),
    IngatLocal(usize, CExpr), UbahLocal(usize, CExpr),
    UbahFieldGlobal(usize, Vec<String>, CExpr), UbahFieldLocal(usize, Vec<String>, CExpr),
    UbahJalurGlobal(usize, Vec<CJalur>, CExpr), UbahJalurLocal(usize, Vec<CJalur>, CExpr),
    Tampilkan(CExpr),
    Kalau(CExpr, Vec<(usize, CStmt)>, Option<Vec<(usize, CStmt)>>),
    Ulang(CExpr, Vec<(usize, CStmt)>),
    UlangSetiapGlobal(usize, CExpr, Vec<(usize, CStmt)>),
    UlangSetiapLocal(usize, CExpr, Vec<(usize, CStmt)>),
    UlangSelaras(CExpr, String, Vec<(usize, Stmt)>),
    CobaGlobal(Vec<(usize, CStmt)>, usize, Vec<(usize, CStmt)>),
    CobaLocal(Vec<(usize, CStmt)>, usize, Vec<(usize, CStmt)>),
    Kembalikan(CExpr),
    EkspresiStmt(CExpr),
    Putus,
    Lanjut,
}

// TipeJit::Campur (BARU) -- fungsi dengan slot BERBEDA tipe (mis. field struct campuran
// Angka+Desimal), TAPI tiap OPERASI individual (BinOp/perbandingan) diverifikasi
// same-type di kedua operand-nya (lewat tipe_cexpr() di bawah) SEBELUM diizinkan JIT --
// kalau ada satu operasi saja yang benar-benar mencampur Angka+Desimal, seluruh fungsi
// GAGAL syarat murni (fallback ke interpreter, aman) -- BUKAN nyoba implementasi promosi
// tipe implisit (int->float) yang lebih riskan salah. Lihat benchmarks/representasi/README.md.
#[derive(Debug, Clone, Copy, PartialEq)]
enum TipeJit { Angka, Desimal, Campur }

fn tipe_jit_dari_nama(s: &str) -> Option<TipeJit> {
    match s { "Angka" => Some(TipeJit::Angka), "Desimal" => Some(TipeJit::Desimal), _ => None }
}

/// Cek apakah sebuah fungsi "murni" secara sintaksis: cuma pakai konstruksi yang bisa
/// diterjemahkan langsung ke instruksi mesin (aritmatika +,-,*, perbandingan, kalau/jika,
/// ulang, dan rekursi ke dirinya sendiri saja). Tanpa pembagian (hindari trap div-by-nol
/// di kode native), tanpa akses variabel global, tanpa panggil fungsi lain/builtin,
/// tanpa Teks/Daftar/Peta. Kalau semua syarat ini + semua slot bertipe Angka terpenuhi,
/// fungsi ini elig dikompilasi ke kode native lewat Cranelift (lihat JitEngine).
/// Scan badan closure (AST mentah, sebelum resolve) buat kumpulkan nama variabel yang DIPAKAI
/// tapi TIDAK diikat (dideklarasikan) di dalam badan itu sendiri -- kandidat variabel yang perlu
/// ditangkap dari scope pembungkus. Boleh over-approximate (misal ikut catat nama fungsi statis
/// yang dipanggil): itu aman, karena keputusan "beneran ditangkap atau enggak" ditentukan belakangan
/// oleh LocalResolver (cuma nama yang memang ada di local_slots scope pembungkus yang ditangkap).
fn variabel_bebas_stmt(s: &Stmt, terikat: &mut std::collections::HashSet<String>, bebas: &mut std::collections::HashSet<String>) {
    match s {
        Stmt::Ingat(nama, _, e) => { variabel_bebas_expr(e, terikat, bebas); terikat.insert(nama.clone()); }
        Stmt::Ubah(nama, e) => { if !terikat.contains(nama) { bebas.insert(nama.clone()); } variabel_bebas_expr(e, terikat, bebas); }
        Stmt::UbahJalur(nama, jalur, e) => {
            if !terikat.contains(nama) { bebas.insert(nama.clone()); }
            for j in jalur { if let Jalur::Indeks(idx) = j { variabel_bebas_expr(idx, terikat, bebas); } }
            variabel_bebas_expr(e, terikat, bebas);
        }
        Stmt::UbahField(nama, _, e) => { if !terikat.contains(nama) { bebas.insert(nama.clone()); } variabel_bebas_expr(e, terikat, bebas); }
        Stmt::BentukDef(..) | Stmt::Muat(..) => {}
        Stmt::Tampilkan(e) => variabel_bebas_expr(e, terikat, bebas),
        Stmt::Kalau(c, tb, eb) => {
            variabel_bebas_expr(c, terikat, bebas);
            for (_, s) in tb { variabel_bebas_stmt(s, terikat, bebas); }
            if let Some(eb) = eb { for (_, s) in eb { variabel_bebas_stmt(s, terikat, bebas); } }
        }
        Stmt::Ulang(c, b) => { variabel_bebas_expr(c, terikat, bebas); for (_, s) in b { variabel_bebas_stmt(s, terikat, bebas); } }
        Stmt::UlangSetiap(var, e, b) | Stmt::UlangSelaras(var, e, b) => {
            variabel_bebas_expr(e, terikat, bebas);
            terikat.insert(var.clone());
            for (_, s) in b { variabel_bebas_stmt(s, terikat, bebas); }
        }
        Stmt::FungsiDef(_, _, _) => { /* fungsi bernama bersarang gak didukung -- biar error normal muncul saat resolve sungguhan */ }
        Stmt::Kembalikan(e) => variabel_bebas_expr(e, terikat, bebas),
        Stmt::EkspresiStmt(e) => variabel_bebas_expr(e, terikat, bebas),
        Stmt::Coba(bc, var, bt) => {
            for (_, s) in bc { variabel_bebas_stmt(s, terikat, bebas); }
            terikat.insert(var.clone());
            for (_, s) in bt { variabel_bebas_stmt(s, terikat, bebas); }
        }
        Stmt::Putus | Stmt::Lanjut => {}
    }
}

fn variabel_bebas_expr(e: &Expr, terikat: &mut std::collections::HashSet<String>, bebas: &mut std::collections::HashSet<String>) {
    match e {
        Expr::Angka(_) | Expr::Desimal(_) | Expr::Teks(_) | Expr::Bool(_) => {}
        Expr::Ident(nama) => { if !terikat.contains(nama) { bebas.insert(nama.clone()); } }
        Expr::Binary(l, _, r) => { variabel_bebas_expr(l, terikat, bebas); variabel_bebas_expr(r, terikat, bebas); }
        Expr::Panggil(nama, args) => {
            // 'nama' bisa jadi nama fungsi statis ATAU variabel closure -- catat aja sebagai
            // kandidat bebas, keputusan akhirnya di LocalResolver (lihat komentar di atas).
            if !terikat.contains(nama) { bebas.insert(nama.clone()); }
            for a in args { variabel_bebas_expr(a, terikat, bebas); }
        }
        Expr::Daftar(items) => { for i in items { variabel_bebas_expr(i, terikat, bebas); } }
        Expr::Peta(entries) => { for (_, v) in entries { variabel_bebas_expr(v, terikat, bebas); } }
        Expr::Indeks(t, i) => { variabel_bebas_expr(t, terikat, bebas); variabel_bebas_expr(i, terikat, bebas); }
        Expr::Field(t, _) => variabel_bebas_expr(t, terikat, bebas),
        Expr::PanggilMetode(t, _, args) => {
            variabel_bebas_expr(t, terikat, bebas);
            for a in args { variabel_bebas_expr(a, terikat, bebas); }
        }
        Expr::Tidak(e) => variabel_bebas_expr(e, terikat, bebas),
        Expr::BentukLiteral(_, entries) => { for (_, v) in entries { variabel_bebas_expr(v, terikat, bebas); } }
        Expr::FungsiLiteral(params, body) => {
            // Closure bersarang lagi di dalam closure -- scope sendiri (parameter closure dalam
            // terikat lokal buat scan ini), tapi badannya tetap bisa merujuk variabel level ini,
            // jadi tetap discan, bukan dilewati.
            let sudah = terikat.clone();
            for (p, _) in params { terikat.insert(p.clone()); }
            for (_, s) in body { variabel_bebas_stmt(s, terikat, bebas); }
            *terikat = sudah;
        }
    }
}

fn cek_jit_murni_stmt(s: &CStmt, nama_sendiri: &str, arity: usize, mode: TipeJit, slot_tipe: &[Option<TipeJit>]) -> bool {
    match s {
        CStmt::IngatLocal(_, e) | CStmt::UbahLocal(_, e) => cek_jit_murni_nilai(e, nama_sendiri, arity, mode),
        CStmt::Kalau(c, tb, eb) => cek_jit_murni_kondisi(c, nama_sendiri, arity, mode, slot_tipe)
            && tb.iter().all(|(_, s)| cek_jit_murni_stmt(s, nama_sendiri, arity, mode, slot_tipe))
            && eb.as_ref().map_or(true, |b| b.iter().all(|(_, s)| cek_jit_murni_stmt(s, nama_sendiri, arity, mode, slot_tipe))),
        CStmt::Ulang(c, b) => cek_jit_murni_kondisi(c, nama_sendiri, arity, mode, slot_tipe) && b.iter().all(|(_, s)| cek_jit_murni_stmt(s, nama_sendiri, arity, mode, slot_tipe)),
        CStmt::Kembalikan(e) => cek_jit_murni_nilai(e, nama_sendiri, arity, mode)
            // Mode Campur: nilai kembalian WAJIB Angka (atau ambigu, default Angka) -- signature
            // Cranelift butuh SATU tipe kembalian pasti, dan validasi_petani-style (kembalikan
            // kode error/status sbg Angka) itu pola yang paling umum. Kalau butuh kembalikan
            // Desimal dari fungsi Campur, itu di luar cakupan slice aman ini (fallback interpreter).
            && (mode != TipeJit::Campur || !matches!(tipe_cexpr(e, slot_tipe), Ok(Some(TipeJit::Desimal)) | Err(()))),
        CStmt::EkspresiStmt(e) => cek_jit_murni_nilai(e, nama_sendiri, arity, mode),
        _ => false, // IngatGlobal/UbahGlobal/UlangSetiap*/UlangSelaras/CobaGlobal/CobaLocal -> bukan fungsi murni
    }
}

/// Tentukan tipe HASIL sebuah CExpr numerik dari sudut pandang tipe slot-nya (`slot_tipe`
/// per-index) -- dipakai KHUSUS buat verifikasi mode TipeJit::Campur (lihat catatan panjang
/// di enum TipeJit): pastikan operand kiri&kanan sebuah perbandingan BENAR-BENAR same-type
/// sebelum diizinkan JIT. Ok(None) = ambigu (literal Angka polos, cocok tipe apa saja) atau
/// di luar cakupan (bukan numerik) -- caller yang memutuskan gimana menyikapi. Err(()) =
/// KONFLIK NYATA (satu sisi Angka, sisi lain Desimal) -- caller HARUS menolak (fallback
/// interpreter, aman) -- SENGAJA tidak nyoba promosi tipe implisit (int->float), itu lebih
/// riskan salah kalau meleset.
fn tipe_cexpr(e: &CExpr, slot_tipe: &[Option<TipeJit>]) -> Result<Option<TipeJit>, ()> {
    match e {
        CExpr::Local(i) => Ok(slot_tipe.get(*i).copied().flatten()),
        CExpr::Angka(_) => Ok(None),
        CExpr::Desimal(_) => Ok(Some(TipeJit::Desimal)),
        CExpr::Binary(l, _, r) => {
            let tl = tipe_cexpr(l, slot_tipe)?;
            let tr = tipe_cexpr(r, slot_tipe)?;
            match (tl, tr) {
                (Some(a), Some(b)) if a == b => Ok(Some(a)),
                (Some(a), None) | (None, Some(a)) => Ok(Some(a)),
                (None, None) => Ok(None),
                (Some(_), Some(_)) => Err(()), // Angka vs Desimal, konflik nyata -- TOLAK
            }
        }
        _ => Ok(None),
    }
}

fn cek_jit_murni_nilai(e: &CExpr, nama_sendiri: &str, arity: usize, mode: TipeJit) -> bool {
    match e {
        CExpr::Local(_) => true,
        // Literal Angka boleh muncul di kedua mode (di mode Desimal ia otomatis dipromosikan
        // ke konstanta f64 saat codegen). Literal Desimal cuma sah kalau mode-nya Desimal.
        CExpr::Angka(_) => true,
        // Literal Desimal valid di mode Desimal ATAU Campur (field bertipe Desimal boleh
        // dibandingkan/diisi literal Desimal, lihat catatan panjang di enum TipeJit) -- cuma
        // ditolak di mode Angka murni (di situ SEMUA slot i64, literal Desimal gak masuk akal).
        CExpr::Desimal(_) => mode != TipeJit::Angka,
        // Campur SENGAJA tidak boleh aritmatika sama sekali (lihat catatan panjang di enum
        // TipeJit) -- cuma dipakai buat PERBANDINGAN (lihat cek_jit_murni_kondisi), yang tidak
        // butuh mekanisme overflow-flag/promosi tipe implisit sama sekali, jauh lebih aman.
        // Modulo KHUSUS diizinkan di mode Angka murni (BUKAN Desimal/Campur) -- butuh
        // mekanisme flag_var/out_ptr buat lapor "modulo dengan nol" balik ke pemanggil (lihat
        // catatan panjang di gabung_flag/tulis_flag_keluaran), yang CUMA ada di mode Angka.
        // Bagi (division) TETAP tidak diizinkan sama sekali -- scope sengaja dipersempit ke
        // Modulo doang (kebutuhan nyata: pola 'i % n' di fungsi pembungkus validasi-style,
        // lihat benchmarks/representasi/README.md).
        CExpr::Binary(l, op, r) => mode != TipeJit::Campur
            && (matches!(op, BinOp::Tambah | BinOp::Kurang | BinOp::Kali) || (matches!(op, BinOp::Modulo) && mode == TipeJit::Angka))
            && cek_jit_murni_nilai(l, nama_sendiri, arity, mode) && cek_jit_murni_nilai(r, nama_sendiri, arity, mode),
        CExpr::Panggil(nama, args) => nama == nama_sendiri && args.len() == arity && args.iter().all(|a| cek_jit_murni_nilai(a, nama_sendiri, arity, mode)),
        _ => false, // Teks/Bool/Global/Daftar/Peta/Indeks/Field/BentukLiteral/Bagi/panggilan-lain -> bukan
    }
}

fn cek_jit_murni_kondisi(e: &CExpr, nama_sendiri: &str, arity: usize, mode: TipeJit, slot_tipe: &[Option<TipeJit>]) -> bool {
    match e {
        CExpr::Binary(l, op, r) => match op {
            BinOp::SamaDengan | BinOp::TidakSama | BinOp::LebihBesar | BinOp::LebihBesarSama
            | BinOp::LebihKecil | BinOp::LebihKecilSama => cek_jit_murni_nilai(l, nama_sendiri, arity, mode) && cek_jit_murni_nilai(r, nama_sendiri, arity, mode)
                // Mode Campur: WAJIB verifikasi operand kiri&kanan same-type (lihat tipe_cexpr()) --
                // mode Angka/Desimal biasa tidak perlu (uniformitas sudah dijamin tipe_seragam
                // di titik lain, lihat resolve_fungsi_umum).
                && (mode != TipeJit::Campur || tipe_cexpr(e, slot_tipe).is_ok()),
            BinOp::Dan | BinOp::Atau => cek_jit_murni_kondisi(l, nama_sendiri, arity, mode, slot_tipe) && cek_jit_murni_kondisi(r, nama_sendiri, arity, mode, slot_tipe),
            _ => false,
        },
        // Kondisi yang sudah terlipat penuh jadi literal Bool oleh optimizer IR (mis. dari
        // `1 < 2` di kode sumber) -- TANPA baris ini, fungsi yang tadinya elig JIT bisa jadi
        // TIDAK elig lagi gara-gara optimizer terlalu pintar (regresi performa, walau tetap
        // benar secara semantik lewat jalur bytecode). Lihat docs/IR.md.
        CExpr::Bool(_) => true,
        _ => false,
    }
}

pub struct CFungsi {
    param_count: usize,
    local_slot_count: usize,
    body: Vec<(usize, CStmt)>,
    // Dipakai JitEngine::kompilasi (nama simbol Cranelift) -- di-gate fitur "jit" saja
    // yang membaca field ini, jadi "tidak pernah dibaca" tanpa fitur itu adalah wajar.
    #[cfg_attr(not(feature = "jit"), allow(dead_code))]
    nama: String,
    slot_tipe: Vec<Option<TipeJit>>,
    /// Some(t) kalau fungsi ini "murni": semua parameter & variabel lokal bertipe SAMA (t),
    /// baik semuanya Angka atau semuanya Desimal (gak campur), tanpa Teks/Bool/Daftar/Peta/Bentuk,
    /// tanpa akses global, tanpa panggil fungsi lain selain dirinya sendiri (rekursi),
    /// tanpa pembagian (hindari trap div-by-zero). Kalau Some, fungsi ini elig dikompilasi
    /// JIT ke kode mesin asli lewat Cranelift, dengan t menentukan tipe Cranelift (I64/F64).
    tipe_jit: Option<TipeJit>,
    /// Per PARAMETER LOGIS (bukan slot -- lihat catatan di param_count), None kalau parameter
    /// itu biasa (1 slot), Some((nama_bentuk, urutan_field)) kalau parameter itu instans 'bentuk'
    /// yang SEMUA field-nya numerik (Angka/Desimal) -- di-"flatten" jadi beberapa slot lokal
    /// berurutan (satu slot per field, tipe Cranelift mengikuti tipe field-nya), diakses langsung
    /// tanpa lookup dinamis. Lihat komentar panjang di resolve_fungsi_umum soal cara kerjanya.
    param_flat: Vec<Option<(String, Vec<String>)>>,
}

/// Inferensi tipe statis sederhana (gradual typing): mencoba menebak tipe hasil sebuah
/// ekspresi HANYA kalau bisa dipastikan tanpa menjalankan program (literal, variabel
/// bertipe diketahui, operasi aritmatika/logika di antara tipe yang diketahui). Kalau
/// tidak bisa dipastikan (misal hasil panggilan fungsi, Daftar, Peta, indeks), kembalikan
/// None -- itulah "gradual": bagian yang bertipe jelas dicek, bagian yang tidak jelas
/// dibiarkan dinamis seperti biasa, tidak dipaksa.
fn infer_tipe(e: &Expr, tipe_var: &HashMap<String, String>) -> Option<String> {
    match e {
        Expr::Angka(_) => Some("Angka".to_string()),
        Expr::Desimal(_) => Some("Desimal".to_string()),
        Expr::Teks(_) => Some("Teks".to_string()),
        Expr::Bool(_) => Some("Bool".to_string()),
        Expr::Ident(nama) => tipe_var.get(nama).cloned(),
        Expr::Binary(l, op, r) => {
            let tl = infer_tipe(l, tipe_var);
            let tr = infer_tipe(r, tipe_var);
            match op {
                BinOp::SamaDengan | BinOp::TidakSama | BinOp::LebihBesar | BinOp::LebihBesarSama
                | BinOp::LebihKecil | BinOp::LebihKecilSama | BinOp::Dan | BinOp::Atau => Some("Bool".to_string()),
                BinOp::Tambah => match (tl.as_deref(), tr.as_deref()) {
                    (Some("Teks"), _) | (_, Some("Teks")) => Some("Teks".to_string()),
                    (Some("Angka"), Some("Angka")) => Some("Angka".to_string()),
                    (Some("Angka"), Some("Desimal")) | (Some("Desimal"), Some("Angka")) | (Some("Desimal"), Some("Desimal")) => Some("Desimal".to_string()),
                    _ => None,
                },
                BinOp::Kurang | BinOp::Kali | BinOp::Bagi | BinOp::Modulo => match (tl.as_deref(), tr.as_deref()) {
                    (Some("Angka"), Some("Angka")) => Some("Angka".to_string()),
                    (Some("Angka"), Some("Desimal")) | (Some("Desimal"), Some("Angka")) | (Some("Desimal"), Some("Desimal")) => Some("Desimal".to_string()),
                    _ => None,
                },
            }
        }
        _ => None, // Panggil, Daftar, Peta, Indeks: tipe hasilnya tidak dicek statis (tetap dinamis)
    }
}

fn cek_tipe(nama: &str, tipe_deklarasi: &str, e: &Expr, tipe_var: &HashMap<String, String>) -> Result<(), String> {
    if let Some(tipe_aktual) = infer_tipe(e, tipe_var) {
        if tipe_aktual != tipe_deklarasi {
            return Err(format!(
                "Kesalahan Tipe: variabel \"{}\" bertipe {}, tapi diberi nilai bertipe {}.",
                nama, tipe_deklarasi, tipe_aktual
            ));
        }
    }
    Ok(())
}

pub struct Resolver {
    global_slots: HashMap<String, usize>, global_count: usize, fungsi_out: HashMap<String, Rc<CFungsi>>, tipe_var: HashMap<String, String>,
    /// Skema tiap 'bentuk' yang dideklarasikan: nama field dalam urutan tetap + tipe opsionalnya.
    /// Dikumpulkan lewat pre-pass di resolve_top() supaya bisa dipakai walau 'bentuk'-nya
    /// dideklarasikan setelah dipakai (forward reference), sama seperti fungsi.
    bentuk_skema: HashMap<String, Vec<(String, Option<String>)>>,
    /// Penghitung buat nama sintetis unik closure ("<closure#N>"), dipakai baik oleh resolver
    /// level atas maupun LocalResolver (closure bersarang) lewat referensi &mut yang sama.
    closure_counter: usize,
    /// Pre-scan (sebelum badan fungsi manapun diresolve, biar forward-reference tetap jalan)
    /// dari nama fungsi -> info parameter mana yang "flattened" (lihat CFungsi::param_flat).
    /// Dipakai saat meresolve PANGGILAN ke fungsi itu (bukan saat meresolve fungsi itu sendiri).
    param_flat_info: HashMap<String, Vec<Option<(String, Vec<String>)>>>,
    /// Berapa lapis 'ulang'/'ulang setiap' yang sedang membungkus statement yang lagi
    /// diresolve -- dipakai buat validasi 'putus'/'lanjut' cuma boleh dipakai di dalam loop.
    /// TIDAK dinaikkan oleh 'ulang selaras' (evaluator terpisah, punya validasi sendiri).
    loop_depth: usize,
}

impl Resolver {
    pub fn new() -> Self { Resolver { global_slots: HashMap::new(), global_count: 0, fungsi_out: HashMap::new(), tipe_var: HashMap::new(), bentuk_skema: HashMap::new(), closure_counter: 0, param_flat_info: HashMap::new(), loop_depth: 0 } }

    fn slot_global(&mut self, nama: &str) -> usize {
        if let Some(&i) = self.global_slots.get(nama) { i } else {
            let i = self.global_count;
            self.global_slots.insert(nama.to_string(), i);
            self.global_count += 1;
            i
        }
    }

    pub fn resolve_top(&mut self, stmts: &[(usize, Stmt)]) -> Result<Vec<(usize, CStmt)>, String> {
        for (_, s) in stmts {
            if let Stmt::BentukDef(nama, fields) = s {
                if self.bentuk_skema.contains_key(nama) {
                    return Err(format!("Bentuk \"{}\" sudah dideklarasikan sebelumnya.", nama));
                }
                let mut nama_field_terlihat = std::collections::HashSet::new();
                for (fnama, _) in fields {
                    if !nama_field_terlihat.insert(fnama) {
                        return Err(format!("Bentuk \"{}\": field \"{}\" dipakai lebih dari sekali.", nama, fnama));
                    }
                }
                self.bentuk_skema.insert(nama.clone(), fields.clone());
            }
        }
        for (_, s) in stmts {
            if let Stmt::FungsiDef(nama, params, _) = s {
                self.param_flat_info.insert(nama.clone(), hitung_param_flat(params, &self.bentuk_skema));
            }
        }
        let mut out = Vec::new();
        for (baris, s) in stmts {
            match s {
                Stmt::FungsiDef(nama, params, body) => {
                    if self.fungsi_out.contains_key(nama) {
                        return Err(format!("Fungsi \"{}\" sudah dideklarasikan sebelumnya.", nama));
                    }
                    let cf = resolve_fungsi_umum(nama, &[], params, body, &self.bentuk_skema, &self.global_slots, &self.param_flat_info, &mut self.fungsi_out, &mut self.closure_counter)?;
                    self.fungsi_out.insert(nama.clone(), Rc::new(cf));
                }
                Stmt::BentukDef(..) => { /* sudah ditangani di pre-pass di atas */ }
                lain => out.push((*baris, self.resolve_stmt_global(lain)?)),
            }
        }
        Ok(out)
    }

    /// Validasi & urutkan field literal 'Nama { ... }' sesuai skema 'bentuk'. Dipakai oleh
    /// resolver global maupun lokal (logikanya sama, cuma resolve_expr-nya beda closure).
    fn urutkan_field_bentuk<'a>(&self, nama: &str, entries: &'a [(String, Expr)]) -> Result<Vec<&'a Expr>, String> {
        let skema = self.bentuk_skema.get(nama)
            .ok_or_else(|| format!("Bentuk \"{}\" tidak dikenal. Apakah sudah dideklarasikan dengan 'bentuk'?", nama))?;
        let mut fnama_terlihat = std::collections::HashSet::new();
        for (fnama, _) in entries {
            if !skema.iter().any(|(sn, _)| sn == fnama) {
                return Err(format!("Bentuk \"{}\" tidak punya field \"{}\".", nama, fnama));
            }
            if !fnama_terlihat.insert(fnama) {
                return Err(format!("Bentuk \"{}\": field \"{}\" diisi lebih dari sekali.", nama, fnama));
            }
        }
        let mut hasil = Vec::with_capacity(skema.len());
        for (sfnama, _) in skema {
            let e = entries.iter().find(|(fnama, _)| fnama == sfnama)
                .map(|(_, e)| e)
                .ok_or_else(|| format!("Bentuk \"{}\" butuh field \"{}\" yang belum diisi.", nama, sfnama))?;
            hasil.push(e);
        }
        Ok(hasil)
    }

    fn resolve_blok_global(&mut self, stmts: &[(usize, Stmt)]) -> Result<Vec<(usize, CStmt)>, String> {
        stmts.iter().map(|(b, s)| Ok((*b, self.resolve_stmt_global(s)?))).collect()
    }

    fn resolve_stmt_global(&mut self, s: &Stmt) -> Result<CStmt, String> {
        match s {
            Stmt::Ingat(nama, tipe, e) => {
                // Deklarasi ulang nama yang SAMA lewat 'ingat' dua kali di scope yang sama
                // dulunya diterima diam-diam (nilai lama ketiban tanpa peringatan) -- itu bug
                // tersembunyi yang gampang kejadian pas copy-paste kode. Sekarang error jelas;
                // kalau memang mau UBAH nilai variabel yang sudah ada, pakai 'nama = nilai'
                // (tanpa 'ingat') -- itu tetap sah dan tidak kena aturan ini.
                if self.global_slots.contains_key(nama) {
                    return Err(format!("Variabel \"{}\" sudah dideklarasikan sebelumnya dengan 'ingat'. Kalau mau mengubah nilainya, pakai '{} = nilai_baru' (tanpa 'ingat').", nama, nama));
                }
                if let Some(t) = tipe {
                    cek_tipe(nama, t, e, &self.tipe_var)?;
                    self.tipe_var.insert(nama.clone(), t.clone());
                } else {
                    self.tipe_var.remove(nama);
                }
                // Kasus khusus 'ingat nama = fungsi(...) {...}': daftarkan slot 'nama' LEBIH
                // DULU (sebelum resolve badan closure-nya), supaya closure top-level bisa
                // rekursi ke dirinya sendiri lewat namanya. Ini aman karena isi slotnya
                // dibaca LIVE lewat Global tiap closure dipanggil (bukan snapshot capture) --
                // closure-nya baru "ada" di slot itu setelah statement ini selesai, tapi
                // pemanggilan diri-sendiri baru kejadian belakangan (saat fungsinya dipanggil).
                let (ce, slot) = if matches!(e, Expr::FungsiLiteral(..)) {
                    let slot = self.slot_global(nama);
                    (self.resolve_expr_global(e)?, slot)
                } else {
                    let ce = self.resolve_expr_global(e)?;
                    (ce, self.slot_global(nama))
                };
                Ok(CStmt::IngatGlobal(slot, ce))
            }
            Stmt::Ubah(nama, e) => {
                if let Some(t) = self.tipe_var.get(nama).cloned() {
                    cek_tipe(nama, &t, e, &self.tipe_var)?;
                }
                let ce = self.resolve_expr_global(e)?;
                let slot = *self.global_slots.get(nama).ok_or_else(|| format!("Variabel \"{}\" belum dideklarasikan dengan 'ingat'.", nama))?;
                Ok(CStmt::UbahGlobal(slot, ce))
            }
            Stmt::Tampilkan(e) => Ok(CStmt::Tampilkan(self.resolve_expr_global(e)?)),
            Stmt::Kalau(cond, tb, eb) => {
                let c = self.resolve_expr_global(cond)?;
                let t = self.resolve_blok_global(tb)?;
                let e = match eb { Some(b) => Some(self.resolve_blok_global(b)?), None => None };
                Ok(CStmt::Kalau(c, t, e))
            }
            Stmt::Ulang(cond, body) => {
                let c = self.resolve_expr_global(cond)?;
                self.loop_depth += 1;
                let b = self.resolve_blok_global(body)?;
                self.loop_depth -= 1;
                Ok(CStmt::Ulang(c, b))
            }
            Stmt::UlangSetiap(var, e, body) => {
                let ce = self.resolve_expr_global(e)?;
                let slot = self.slot_global(var);
                self.loop_depth += 1;
                let b = self.resolve_blok_global(body)?;
                self.loop_depth -= 1;
                Ok(CStmt::UlangSetiapGlobal(slot, ce, b))
            }
            Stmt::UlangSelaras(var, e, body) => {
                validasi_tubuh_selaras(body)?;
                let ce = self.resolve_expr_global(e)?;
                Ok(CStmt::UlangSelaras(ce, var.clone(), body.clone()))
            }
            Stmt::Coba(badan_coba, nama_var, badan_tangkap) => {
                let bc = self.resolve_blok_global(badan_coba)?;
                let slot = self.slot_global(nama_var);
                let bt = self.resolve_blok_global(badan_tangkap)?;
                Ok(CStmt::CobaGlobal(bc, slot, bt))
            }
            Stmt::Kembalikan(e) => Ok(CStmt::Kembalikan(self.resolve_expr_global(e)?)),
            Stmt::EkspresiStmt(e) => Ok(CStmt::EkspresiStmt(self.resolve_expr_global(e)?)),
            Stmt::UbahField(nama, field, e) => {
                let ce = self.resolve_expr_global(e)?;
                let slot = *self.global_slots.get(nama).ok_or_else(|| format!("Variabel \"{}\" belum dideklarasikan dengan 'ingat'.", nama))?;
                Ok(CStmt::UbahFieldGlobal(slot, field.clone(), ce))
            }
            Stmt::UbahJalur(nama, jalur, e) => {
                let ce = self.resolve_expr_global(e)?;
                let cjalur: Vec<CJalur> = jalur.iter().map(|j| Ok(match j {
                    Jalur::Field(f) => CJalur::Field(f.clone()),
                    Jalur::Indeks(idx) => CJalur::Indeks(self.resolve_expr_global(idx)?),
                })).collect::<Result<_, String>>()?;
                let slot = *self.global_slots.get(nama).ok_or_else(|| format!("Variabel \"{}\" belum dideklarasikan dengan 'ingat'.", nama))?;
                Ok(CStmt::UbahJalurGlobal(slot, cjalur, ce))
            }
            Stmt::FungsiDef(..) | Stmt::BentukDef(..) => unreachable!(),
            Stmt::Muat(..) => Err("'muat' cuma boleh dipakai di level atas program (bukan di dalam kalau/ulang).".to_string()),
            Stmt::Putus => {
                if self.loop_depth == 0 { return Err("'putus' hanya boleh dipakai di dalam 'ulang' atau 'ulang setiap'.".to_string()); }
                Ok(CStmt::Putus)
            }
            Stmt::Lanjut => {
                if self.loop_depth == 0 { return Err("'lanjut' hanya boleh dipakai di dalam 'ulang' atau 'ulang setiap'.".to_string()); }
                Ok(CStmt::Lanjut)
            }
        }
    }

    fn resolve_expr_global(&mut self, e: &Expr) -> Result<CExpr, String> {
        match e {
            Expr::Angka(n) => Ok(CExpr::Angka(*n)),
            Expr::Desimal(f) => Ok(CExpr::Desimal(*f)),
            Expr::Teks(s) => Ok(CExpr::Teks(s.clone())),
            Expr::Bool(b) => Ok(CExpr::Bool(*b)),
            Expr::Ident(nama) => {
                let slot = *self.global_slots.get(nama).ok_or_else(|| format!("Variabel \"{}\" tidak ditemukan. Apakah sudah dideklarasikan dengan 'ingat'?", nama))?;
                Ok(CExpr::Global(slot))
            }
            Expr::Binary(l, op, r) => Ok(CExpr::Binary(Box::new(self.resolve_expr_global(l)?), *op, Box::new(self.resolve_expr_global(r)?))),
            Expr::Panggil(nama, args) => {
                if let Some(&slot) = self.global_slots.get(nama) {
                    // Ada variabel global bernama sama -- anggap itu closure yang mau dipanggil
                    // sebagai NILAI (bukan nama fungsi statis). Konsisten dgn aturan shadowing biasa.
                    let cargs: Vec<CExpr> = args.iter().map(|a| self.resolve_expr_global(a)).collect::<Result<_, _>>()?;
                    Ok(CExpr::PanggilNilai(Box::new(CExpr::Global(slot)), cargs))
                } else {
                    let pfi = self.param_flat_info.clone();
                    let info = pfi.get(nama);
                    let mut cargs = Vec::new();
                    for (i, a) in args.iter().enumerate() {
                        let flat = info.and_then(|v| v.get(i)).and_then(|o| o.as_ref());
                        match flat {
                            Some((struct_nama, field_urut)) => {
                                if let Expr::Ident(var_nama) = a {
                                    for fnama in field_urut {
                                        let fe = Expr::Field(Box::new(Expr::Ident(var_nama.clone())), fnama.clone());
                                        cargs.push(self.resolve_expr_global(&fe)?);
                                    }
                                } else if let Expr::BentukLiteral(lit_nama, entries) = a {
                                    // Fast-path: argumen literal Bentuk LANGSUNG di titik panggil
                                    // (mis. 'f(Titik{x:3,y:4})') -- SKIP konstruksi Instans dinamis
                                    // sepenuhnya, langsung pakai ekspresi field-nya sebagai argumen.
                                    // Sebelum ada ini, 'Titik{x:3,y:4}' tetap dibangun jadi Instans
                                    // (BuatInstans, alokasi heap) lalu LANGSUNG dibongkar lagi lewat
                                    // SimpanLaluField -- kerja dua kali buat objek yang cuma hidup
                                    // sepersekian detik. Field HARUS diurutkan sesuai skema Bentuk
                                    // (bukan urutan penulisan user), makanya lewat
                                    // urutkan_field_bentuk() -- lihat juga cek di bawah kalau nama
                                    // bentuk yang dipanggil beda dari yang dideklarasikan.
                                    if lit_nama != struct_nama {
                                        return Err(format!("Argumen ke-{} fungsi \"{}\" butuh bentuk \"{}\", ditemukan \"{}\".", i + 1, nama, struct_nama, lit_nama));
                                    }
                                    let terurut = self.urutkan_field_bentuk(lit_nama, entries)?;
                                    for fe in terurut {
                                        cargs.push(self.resolve_expr_global(fe)?);
                                    }
                                } else if field_urut.is_empty() {
                                    self.resolve_expr_global(a)?;
                                } else {
                                    let ce_instans = self.resolve_expr_global(a)?;
                                    self.closure_counter += 1;
                                    let nama_slot = format!("<tmp#{}>", self.closure_counter);
                                    let slot = self.slot_global(&nama_slot);
                                    cargs.push(CExpr::SimpanLaluField(Box::new(ce_instans), SlotSasaran::Global(slot), field_urut[0].clone()));
                                    for fnama in &field_urut[1..] {
                                        cargs.push(CExpr::Field(Box::new(CExpr::Global(slot)), fnama.clone()));
                                    }
                                }
                            }
                            None => cargs.push(self.resolve_expr_global(a)?),
                        }
                    }
                    Ok(CExpr::Panggil(nama.clone(), cargs))
                }
            }
            Expr::Daftar(items) => Ok(CExpr::Daftar(items.iter().map(|i| self.resolve_expr_global(i)).collect::<Result<_, _>>()?)),
            Expr::Peta(entries) => {
                let mut out = Vec::new();
                for (k, v) in entries { out.push((k.clone(), self.resolve_expr_global(v)?)); }
                Ok(CExpr::Peta(out))
            }
            Expr::Indeks(t, i) => Ok(CExpr::Indeks(Box::new(self.resolve_expr_global(t)?), Box::new(self.resolve_expr_global(i)?))),
            Expr::Field(t, f) => Ok(CExpr::Field(Box::new(self.resolve_expr_global(t)?), f.clone())),
            // 'x.y(args)' yang BUKAN alias modul (sudah ditulis-ulang jadi Expr::Panggil biasa
            // sebelum resolver ini jalan, lihat tulis_ulang_panggil_alias()) -- di sini artinya
            // "baca field y dari x, panggil NILAINYA sebagai fungsi" (mis. closure di field bentuk).
            Expr::PanggilMetode(t, f, args) => {
                let cargs: Vec<CExpr> = args.iter().map(|a| self.resolve_expr_global(a)).collect::<Result<_, _>>()?;
                Ok(CExpr::PanggilNilai(Box::new(CExpr::Field(Box::new(self.resolve_expr_global(t)?), f.clone())), cargs))
            }
            Expr::Tidak(e) => Ok(CExpr::Tidak(Box::new(self.resolve_expr_global(e)?))),
            Expr::BentukLiteral(nama, entries) => {
                let terurut = self.urutkan_field_bentuk(nama, entries)?;
                let skema = self.bentuk_skema.get(nama).unwrap().clone();
                let mut out = Vec::with_capacity(terurut.len());
                for (e, (fnama, ftipe)) in terurut.into_iter().zip(skema.iter()) {
                    if let Some(t) = ftipe { cek_tipe(fnama, t, e, &self.tipe_var)?; }
                    out.push((fnama.clone(), self.resolve_expr_global(e)?));
                }
                Ok(CExpr::BentukLiteral(nama.clone(), out))
            }
            Expr::FungsiLiteral(params, body) => {
                self.closure_counter += 1;
                let nama_sintetis = format!("<closure#{}>", self.closure_counter);
                let cf = resolve_fungsi_umum(&nama_sintetis, &[], params, body, &self.bentuk_skema, &self.global_slots, &self.param_flat_info, &mut self.fungsi_out, &mut self.closure_counter)?;
                self.fungsi_out.insert(nama_sintetis.clone(), Rc::new(cf));
                Ok(CExpr::FungsiLiteral(nama_sintetis, Vec::new()))
            }
        }
    }

}

/// Resolve isi SATU fungsi jadi CFungsi siap-compile -- dipakai baik buat 'fungsi' bernama
/// biasa (tangkapan_nama kosong) MAUPUN closure/fungsi anonim (tangkapan_nama berisi nama
/// variabel yang ditangkap dari scope pembungkus). Tangkapan menempati slot lokal PALING AWAL
/// (0..K-1), baru disusul parameter eksplisit (K..K+P-1) -- jadi dari sudut pandang konvensi
/// panggilan, tangkapan+parameter itu SATU larik argumen yang sama seperti fungsi biasa.
/// Sengaja fungsi bebas (bukan method Resolver) supaya bisa dipanggil dari dalam LocalResolver
/// (buat closure bersarang) tanpa konflik peminjaman &mut self.
/// Untuk tiap parameter di `params`, tentukan apakah ia "flattenable": tipe-nya menunjuk ke
/// sebuah 'bentuk' yang SEMUA field-nya bertipe numerik (Angka/Desimal). Kalau ya, hasilkan
/// Some((nama_bentuk, urutan_field)); kalau bukan (bukan bentuk, bentuk gak dikenal, atau ada
/// field non-numerik), None -- parameter itu tetap 1 slot biasa (instans 'bentuk' opak, akses
/// field lewat jalur dinamis seperti biasa, gak dapat percepatan JIT tapi tetap benar).
fn hitung_param_flat(
    params: &[(String, Option<String>)],
    bentuk_skema: &HashMap<String, Vec<(String, Option<String>)>>,
) -> Vec<Option<(String, Vec<String>)>> {
    params.iter().map(|(_, ptipe)| {
        let tipe = ptipe.as_ref()?;
        let skema = bentuk_skema.get(tipe)?;
        let semua_numerik = skema.iter().all(|(_, ftipe)| ftipe.as_deref().and_then(tipe_jit_dari_nama).is_some());
        if semua_numerik {
            Some((tipe.clone(), skema.iter().map(|(fnama, _)| fnama.clone()).collect()))
        } else {
            None
        }
    }).collect()
}


fn resolve_fungsi_umum(
    nama_fungsi: &str,
    tangkapan_nama: &[String],
    params: &[(String, Option<String>)],
    body: &[(usize, Stmt)],
    bentuk_skema: &HashMap<String, Vec<(String, Option<String>)>>,
    global_slots: &HashMap<String, usize>,
    param_flat_info: &HashMap<String, Vec<Option<(String, Vec<String>)>>>,
    fungsi_out: &mut HashMap<String, Rc<CFungsi>>,
    closure_counter: &mut usize,
) -> Result<CFungsi, String> {
    let mut local_slots: HashMap<String, usize> = HashMap::new();
    let mut struct_params: HashMap<String, (usize, Vec<String>)> = HashMap::new();
    let mut slot_tipe: Vec<Option<TipeJit>> = Vec::new();
    let mut tipe_var: HashMap<String, String> = HashMap::new();
    let mut local_count = 0usize;
    for tnama in tangkapan_nama {
        local_slots.insert(tnama.clone(), local_count);
        slot_tipe.push(None); // tipe tangkapan gak dilacak eksplisit -- kalau ada tangkapan, fungsi ini otomatis gak eligible JIT (lihat tipe_seragam di bawah)
        local_count += 1;
    }
    // Info parameter mana yang "flattened" (instans bentuk numerik-murni, lihat CFungsi::param_flat)
    // -- untuk fungsi bernama biasa, ini sudah di-pre-scan (forward-reference aman); closure gak
    // pernah muncul di pre-scan itu, jadi defaultnya semua None (closure gak dukung flattening).
    let param_flat: Vec<Option<(String, Vec<String>)>> = param_flat_info.get(nama_fungsi).cloned()
        .unwrap_or_else(|| vec![None; params.len()]);
    for ((pnama, ptipe), flat) in params.iter().zip(param_flat.iter()) {
        if local_slots.contains_key(pnama) || struct_params.contains_key(pnama) {
            return Err(format!("Fungsi \"{}\": parameter \"{}\" dipakai lebih dari sekali.", nama_fungsi, pnama));
        }
        match flat {
            Some((bentuk_nama, field_urut)) => {
                // Parameter instans 'bentuk' numerik-murni -- "flatten" jadi beberapa slot lokal
                // berurutan (1 slot per field), bukan 1 slot berisi Value::Instans opak. Field-nya
                // cuma bisa diakses lewat 'param.field' (lihat resolve Expr::Field di bawah) --
                // nama parameternya sendiri SENGAJA gak didaftarkan sebagai variabel biasa,
                // jadi 'param' telanjang (tanpa .field) gak sah dipakai.
                let base = local_count;
                let skema = bentuk_skema.get(bentuk_nama)
                    .ok_or_else(|| format!("Fungsi \"{}\": bentuk \"{}\" tidak dikenal.", nama_fungsi, bentuk_nama))?;
                for fnama in field_urut {
                    let ftipe = skema.iter().find(|(sn, _)| sn == fnama).and_then(|(_, t)| t.as_deref()).and_then(tipe_jit_dari_nama);
                    slot_tipe.push(ftipe);
                    local_count += 1;
                }
                struct_params.insert(pnama.clone(), (base, field_urut.clone()));
            }
            None => {
                local_slots.insert(pnama.clone(), local_count);
                slot_tipe.push(ptipe.as_deref().and_then(tipe_jit_dari_nama));
                if let Some(t) = ptipe { tipe_var.insert(pnama.clone(), t.clone()); }
                local_count += 1;
            }
        }
    }
    let param_count = local_count; // total SLOT (bukan parameter logis) yang diisi pemanggil
    let mut lr = LocalResolver { local_slots, struct_params, local_count, tipe_var, slot_tipe, bentuk_skema, global_slots, param_flat_info, fungsi_out, closure_counter, loop_depth: 0 };
    let cbody = lr.resolve_block(body)?;

    let tipe_seragam: Option<TipeJit> = if param_count == 0 {
        None
    } else if lr.slot_tipe.iter().all(|t| *t == Some(TipeJit::Angka)) {
        Some(TipeJit::Angka)
    } else if lr.slot_tipe.iter().all(|t| *t == Some(TipeJit::Desimal)) {
        Some(TipeJit::Desimal)
    } else if lr.slot_tipe.iter().all(|t| t.is_some()) {
        // Semua slot punya tipe (Angka/Desimal), TAPI tidak seragam -- mis. field struct
        // campuran (bentuk DataPetani { nama_kosong: Angka, lahan: Desimal, ... }). Lihat
        // catatan panjang di enum TipeJit::Campur & cek_jit_murni_kondisi/tipe_cexpr --
        // verifikasi per-operasi (bukan cuma per-fungsi) menjamin ini tetap aman.
        Some(TipeJit::Campur)
    } else {
        None
    };
    let tipe_jit = tipe_seragam.filter(|t| cbody.iter().all(|(_, s)| cek_jit_murni_stmt(s, nama_fungsi, param_count, *t, &lr.slot_tipe)));

    if std::env::var("ISOTERI_DEBUG_JIT").is_ok() {
        eprintln!("DEBUG_JIT fungsi={} tipe_seragam={:?} tipe_jit_final={:?} slot_tipe={:?}", nama_fungsi, tipe_seragam, tipe_jit, lr.slot_tipe);
    }
    Ok(CFungsi {
        param_count,
        local_slot_count: lr.local_count,
        body: cbody,
        nama: nama_fungsi.to_string(),
        slot_tipe: lr.slot_tipe,
        tipe_jit,
        param_flat,
    })
}

struct LocalResolver<'a> {
    local_slots: HashMap<String, usize>, local_count: usize, tipe_var: HashMap<String, String>, slot_tipe: Vec<Option<TipeJit>>,
    /// Parameter fungsi INI SENDIRI yang "flattened" (lihat CFungsi::param_flat) -- nama param ->
    /// (slot dasar, urutan nama field). Field-nya diakses langsung lewat slot (base+indeks_field),
    /// param-nya SENGAJA gak masuk `local_slots` (bare identifier tanpa .field gak sah dipakai).
    struct_params: HashMap<String, (usize, Vec<String>)>,
    bentuk_skema: &'a HashMap<String, Vec<(String, Option<String>)>>,
    /// Fallback baca variabel GLOBAL dari dalam badan fungsi/closure (sebelumnya gak didukung
    /// sama sekali). Global harus sudah dideklarasikan LEBIH DULU secara tekstual (sama seperti
    /// aturan 'ingat' di level atas -- gak ada forward-reference buat variabel, beda dari
    /// fungsi/bentuk yang di-pre-pass).
    global_slots: &'a HashMap<String, usize>,
    /// Info parameter "flattened" milik SEMUA fungsi bernama (bukan cuma fungsi ini sendiri) --
    /// dipakai saat meresolve PANGGILAN ke fungsi lain, biar argumen di posisi yang di-flatten
    /// bisa dipecah jadi beberapa CExpr::Field/Local, bukan 1 nilai instans opak.
    param_flat_info: &'a HashMap<String, Vec<Option<(String, Vec<String>)>>>,
    /// Dua field ini dibawa turun dari Resolver level atas (lewat resolve_fungsi_umum) supaya
    /// closure yang didefinisikan DI DALAM fungsi ini juga bisa terdaftar dgn nama sintetis unik.
    fungsi_out: &'a mut HashMap<String, Rc<CFungsi>>,
    closure_counter: &'a mut usize,
    /// Sama seperti Resolver::loop_depth (lihat catatan di sana), versi lokal buat badan fungsi.
    loop_depth: usize,
}

impl<'a> LocalResolver<'a> {
    fn urutkan_field_bentuk<'b>(&self, nama: &str, entries: &'b [(String, Expr)]) -> Result<Vec<&'b Expr>, String> {
        let skema = self.bentuk_skema.get(nama)
            .ok_or_else(|| format!("Bentuk \"{}\" tidak dikenal. Apakah sudah dideklarasikan dengan 'bentuk'?", nama))?;
        let mut fnama_terlihat = std::collections::HashSet::new();
        for (fnama, _) in entries {
            if !skema.iter().any(|(sn, _)| sn == fnama) {
                return Err(format!("Bentuk \"{}\" tidak punya field \"{}\".", nama, fnama));
            }
            if !fnama_terlihat.insert(fnama) {
                return Err(format!("Bentuk \"{}\": field \"{}\" diisi lebih dari sekali.", nama, fnama));
            }
        }
        let mut hasil = Vec::with_capacity(skema.len());
        for (sfnama, _) in skema {
            let e = entries.iter().find(|(fnama, _)| fnama == sfnama)
                .map(|(_, e)| e)
                .ok_or_else(|| format!("Bentuk \"{}\" butuh field \"{}\" yang belum diisi.", nama, sfnama))?;
            hasil.push(e);
        }
        Ok(hasil)
    }
    fn slot_local(&mut self, nama: &str) -> usize {
        if let Some(&i) = self.local_slots.get(nama) { i } else {
            let i = self.local_count;
            self.local_slots.insert(nama.to_string(), i);
            self.local_count += 1;
            self.slot_tipe.push(None);
            i
        }
    }
    fn resolve_block(&mut self, stmts: &[(usize, Stmt)]) -> Result<Vec<(usize, CStmt)>, String> {
        stmts.iter().map(|(b, s)| Ok((*b, self.resolve_stmt(s)?))).collect()
    }
    fn resolve_stmt(&mut self, s: &Stmt) -> Result<CStmt, String> {
        match s {
            Stmt::Ingat(nama, tipe, e) => {
                // Sama seperti versi global resolver -- lihat catatan lengkap di sana.
                if self.local_slots.contains_key(nama) {
                    return Err(format!("Variabel \"{}\" sudah dideklarasikan sebelumnya dengan 'ingat' (atau merupakan nama parameter fungsi). Kalau mau mengubah nilainya, pakai '{} = nilai_baru' (tanpa 'ingat').", nama, nama));
                }
                if let Some(t) = tipe {
                    cek_tipe(nama, t, e, &self.tipe_var)?;
                    self.tipe_var.insert(nama.clone(), t.clone());
                } else {
                    self.tipe_var.remove(nama);
                }
                let ce = self.resolve_expr(e)?;
                let slot = self.slot_local(nama);
                if let Some(t) = tipe { self.slot_tipe[slot] = tipe_jit_dari_nama(t); }
                Ok(CStmt::IngatLocal(slot, ce))
            }
            Stmt::Ubah(nama, e) => {
                if let Some(t) = self.tipe_var.get(nama).cloned() {
                    cek_tipe(nama, &t, e, &self.tipe_var)?;
                }
                let ce = self.resolve_expr(e)?;
                if let Some(&slot) = self.local_slots.get(nama) {
                    Ok(CStmt::UbahLocal(slot, ce))
                } else if let Some(&slot) = self.global_slots.get(nama) {
                    Ok(CStmt::UbahGlobal(slot, ce))
                } else {
                    Err(format!("Variabel \"{}\" belum dideklarasikan dengan 'ingat'.", nama))
                }
            }
            Stmt::Tampilkan(e) => Ok(CStmt::Tampilkan(self.resolve_expr(e)?)),
            Stmt::Kalau(cond, tb, eb) => {
                let c = self.resolve_expr(cond)?;
                let t = self.resolve_block(tb)?;
                let e = match eb { Some(b) => Some(self.resolve_block(b)?), None => None };
                Ok(CStmt::Kalau(c, t, e))
            }
            Stmt::Ulang(cond, body) => {
                let c = self.resolve_expr(cond)?;
                self.loop_depth += 1;
                let b = self.resolve_block(body)?;
                self.loop_depth -= 1;
                Ok(CStmt::Ulang(c, b))
            }
            Stmt::UlangSetiap(var, e, body) => {
                let ce = self.resolve_expr(e)?;
                let slot = self.slot_local(var);
                self.loop_depth += 1;
                let b = self.resolve_block(body)?;
                self.loop_depth -= 1;
                Ok(CStmt::UlangSetiapLocal(slot, ce, b))
            }
            Stmt::UlangSelaras(var, e, body) => {
                validasi_tubuh_selaras(body)?;
                let ce = self.resolve_expr(e)?;
                Ok(CStmt::UlangSelaras(ce, var.clone(), body.clone()))
            }
            Stmt::Coba(badan_coba, nama_var, badan_tangkap) => {
                let bc = self.resolve_block(badan_coba)?;
                let slot = self.slot_local(nama_var);
                let bt = self.resolve_block(badan_tangkap)?;
                Ok(CStmt::CobaLocal(bc, slot, bt))
            }
            Stmt::Kembalikan(e) => Ok(CStmt::Kembalikan(self.resolve_expr(e)?)),
            Stmt::EkspresiStmt(e) => Ok(CStmt::EkspresiStmt(self.resolve_expr(e)?)),
            Stmt::UbahField(nama, field, e) => {
                let ce = self.resolve_expr(e)?;
                if let Some(&slot) = self.local_slots.get(nama) {
                    Ok(CStmt::UbahFieldLocal(slot, field.clone(), ce))
                } else if let Some(&slot) = self.global_slots.get(nama) {
                    Ok(CStmt::UbahFieldGlobal(slot, field.clone(), ce))
                } else {
                    Err(format!("Variabel \"{}\" belum dideklarasikan dengan 'ingat'.", nama))
                }
            }
            Stmt::UbahJalur(nama, jalur, e) => {
                let ce = self.resolve_expr(e)?;
                let cjalur: Vec<CJalur> = jalur.iter().map(|j| Ok(match j {
                    Jalur::Field(f) => CJalur::Field(f.clone()),
                    Jalur::Indeks(idx) => CJalur::Indeks(self.resolve_expr(idx)?),
                })).collect::<Result<_, String>>()?;
                if let Some(&slot) = self.local_slots.get(nama) {
                    Ok(CStmt::UbahJalurLocal(slot, cjalur, ce))
                } else if let Some(&slot) = self.global_slots.get(nama) {
                    Ok(CStmt::UbahJalurGlobal(slot, cjalur, ce))
                } else {
                    Err(format!("Variabel \"{}\" belum dideklarasikan dengan 'ingat'.", nama))
                }
            }
            Stmt::FungsiDef(..) => Err("Fungsi di dalam fungsi belum didukung di Fase ini.".to_string()),
            Stmt::BentukDef(..) => Err("'bentuk' hanya boleh dideklarasikan di level atas program.".to_string()),
            Stmt::Muat(..) => Err("'muat' cuma boleh dipakai di level atas program (bukan di dalam fungsi).".to_string()),
            Stmt::Putus => {
                if self.loop_depth == 0 { return Err("'putus' hanya boleh dipakai di dalam 'ulang' atau 'ulang setiap'.".to_string()); }
                Ok(CStmt::Putus)
            }
            Stmt::Lanjut => {
                if self.loop_depth == 0 { return Err("'lanjut' hanya boleh dipakai di dalam 'ulang' atau 'ulang setiap'.".to_string()); }
                Ok(CStmt::Lanjut)
            }
        }
    }
    fn resolve_expr(&mut self, e: &Expr) -> Result<CExpr, String> {
        match e {
            Expr::Angka(n) => Ok(CExpr::Angka(*n)),
            Expr::Desimal(f) => Ok(CExpr::Desimal(*f)),
            Expr::Teks(s) => Ok(CExpr::Teks(s.clone())),
            Expr::Bool(b) => Ok(CExpr::Bool(*b)),
            Expr::Ident(nama) => {
                if let Some(&slot) = self.local_slots.get(nama) {
                    Ok(CExpr::Local(slot))
                } else if let Some(&slot) = self.global_slots.get(nama) {
                    Ok(CExpr::Global(slot))
                } else if self.struct_params.contains_key(nama) {
                    Err(format!("Parameter \"{}\" adalah bentuk yang di-flatten untuk performa -- cuma bisa dipakai lewat \"{}.nama_field\", gak bisa dipakai sebagai nilai utuh.", nama, nama))
                } else {
                    Err(format!("Variabel \"{}\" tidak ditemukan (bukan parameter/lokal fungsi ini, bukan juga variabel global).", nama))
                }
            }
            Expr::Binary(l, op, r) => Ok(CExpr::Binary(Box::new(self.resolve_expr(l)?), *op, Box::new(self.resolve_expr(r)?))),
            Expr::Panggil(nama, args) => {
                if let Some(&slot) = self.local_slots.get(nama) {
                    let cargs: Vec<CExpr> = args.iter().map(|a| self.resolve_expr(a)).collect::<Result<_, _>>()?;
                    Ok(CExpr::PanggilNilai(Box::new(CExpr::Local(slot)), cargs))
                } else if let Some(&slot) = self.global_slots.get(nama) {
                    let cargs: Vec<CExpr> = args.iter().map(|a| self.resolve_expr(a)).collect::<Result<_, _>>()?;
                    Ok(CExpr::PanggilNilai(Box::new(CExpr::Global(slot)), cargs))
                } else {
                    let pfi = self.param_flat_info.clone();
                    let info = pfi.get(nama);
                    let mut cargs = Vec::new();
                    for (i, a) in args.iter().enumerate() {
                        let flat = info.and_then(|v| v.get(i)).and_then(|o| o.as_ref());
                        match flat {
                            Some((struct_nama, field_urut)) => {
                                if let Expr::Ident(var_nama) = a {
                                    for fnama in field_urut {
                                        let fe = Expr::Field(Box::new(Expr::Ident(var_nama.clone())), fnama.clone());
                                        cargs.push(self.resolve_expr(&fe)?);
                                    }
                                } else if let Expr::BentukLiteral(lit_nama, entries) = a {
                                    // Fast-path yang sama seperti versi global (lihat komentar
                                    // panjang di resolve_expr_global) -- skip konstruksi Instans
                                    // dinamis buat literal Bentuk langsung di titik panggil.
                                    if lit_nama != struct_nama {
                                        return Err(format!("Argumen ke-{} fungsi \"{}\" butuh bentuk \"{}\", ditemukan \"{}\".", i + 1, nama, struct_nama, lit_nama));
                                    }
                                    let terurut = self.urutkan_field_bentuk(lit_nama, entries)?;
                                    for fe in terurut {
                                        cargs.push(self.resolve_expr(fe)?);
                                    }
                                } else if field_urut.is_empty() {
                                    self.resolve_expr(a)?;
                                } else {
                                    let ce_instans = self.resolve_expr(a)?;
                                    let slot = self.local_count;
                                    self.slot_tipe.push(None); // temp menampung instans 'bentuk', bukan skalar -- otomatis gak seragam-tipe (JIT pemanggil ini nonaktif kalau lewat jalur ini)
                                    self.local_count += 1;
                                    cargs.push(CExpr::SimpanLaluField(Box::new(ce_instans), SlotSasaran::Lokal(slot), field_urut[0].clone()));
                                    for fnama in &field_urut[1..] {
                                        cargs.push(CExpr::Field(Box::new(CExpr::Local(slot)), fnama.clone()));
                                    }
                                }
                            }
                            None => cargs.push(self.resolve_expr(a)?),
                        }
                    }
                    Ok(CExpr::Panggil(nama.clone(), cargs))
                }
            }
            Expr::Daftar(items) => Ok(CExpr::Daftar(items.iter().map(|i| self.resolve_expr(i)).collect::<Result<_, _>>()?)),
            Expr::Peta(entries) => {
                let mut out = Vec::new();
                for (k, v) in entries { out.push((k.clone(), self.resolve_expr(v)?)); }
                Ok(CExpr::Peta(out))
            }
            Expr::Indeks(t, i) => Ok(CExpr::Indeks(Box::new(self.resolve_expr(t)?), Box::new(self.resolve_expr(i)?))),
            Expr::Field(t, f) => {
                // Kalau basisnya identifier telanjang & itu parameter "flattened" milik fungsi
                // ini sendiri, akses field-nya langsung ke slot lokal (murah, ramah-JIT) --
                // bukan lewat CExpr::Field dinamis. Lihat komentar struct_params & param_flat.
                if let Expr::Ident(nama) = t.as_ref() {
                    if let Some((base, field_urut)) = self.struct_params.get(nama) {
                        let idx = field_urut.iter().position(|fn_| fn_ == f)
                            .ok_or_else(|| format!("Bentuk parameter \"{}\" tidak punya field \"{}\".", nama, f))?;
                        return Ok(CExpr::Local(base + idx));
                    }
                }
                Ok(CExpr::Field(Box::new(self.resolve_expr(t)?), f.clone()))
            }
            Expr::PanggilMetode(t, f, args) => {
                let cargs: Vec<CExpr> = args.iter().map(|a| self.resolve_expr(a)).collect::<Result<_, _>>()?;
                Ok(CExpr::PanggilNilai(Box::new(CExpr::Field(Box::new(self.resolve_expr(t)?), f.clone())), cargs))
            }
            Expr::Tidak(e) => Ok(CExpr::Tidak(Box::new(self.resolve_expr(e)?))),
            Expr::BentukLiteral(nama, entries) => {
                let terurut = self.urutkan_field_bentuk(nama, entries)?;
                let skema = self.bentuk_skema.get(nama).unwrap().clone();
                let mut out = Vec::with_capacity(terurut.len());
                for (e, (fnama, ftipe)) in terurut.into_iter().zip(skema.iter()) {
                    if let Some(t) = ftipe { cek_tipe(fnama, t, e, &self.tipe_var)?; }
                    out.push((fnama.clone(), self.resolve_expr(e)?));
                }
                Ok(CExpr::BentukLiteral(nama.clone(), out))
            }
            Expr::FungsiLiteral(params, body) => {
                // 1. Cari variabel bebas di badan closure (dipakai tapi bukan parameter closure
                //    sendiri / bukan dideklarasikan sendiri di dalam badannya).
                let mut terikat: std::collections::HashSet<String> = params.iter().map(|(n, _)| n.clone()).collect();
                let mut bebas: std::collections::HashSet<String> = std::collections::HashSet::new();
                for (_, s) in body { variabel_bebas_stmt(s, &mut terikat, &mut bebas); }

                // 2. Dari variabel bebas itu, yang beneran perlu ditangkap cuma yang memang ADA
                //    di local_slots milik fungsi/closure PEMBUNGKUS ini (kalau bukan, nanti coba
                //    diresolve sebagai fungsi statis/builtin seperti biasa saat badan closure-nya
                //    sendiri diresolve secara rekursif).
                let mut tangkapan_nama: Vec<String> = Vec::new();
                let mut tangkapan_slot_induk: Vec<usize> = Vec::new();
                for nama in &bebas {
                    if let Some(&slot) = self.local_slots.get(nama) {
                        tangkapan_nama.push(nama.clone());
                        tangkapan_slot_induk.push(slot);
                    }
                }

                // 3. Resolve badan closure jadi CFungsi baru & daftarkan dgn nama sintetis unik.
                *self.closure_counter += 1;
                let nama_sintetis = format!("<closure#{}>", *self.closure_counter);
                let cf = resolve_fungsi_umum(&nama_sintetis, &tangkapan_nama, params, body, self.bentuk_skema, self.global_slots, self.param_flat_info, self.fungsi_out, self.closure_counter)?;
                self.fungsi_out.insert(nama_sintetis.clone(), Rc::new(cf));

                // 4. Ekspresi buat ambil nilai tangkapan SAAT INI (snapshot dari slot lokal milik
                //    fungsi pembungkus, dievaluasi di titik literal closure-nya muncul).
                let tangkapan_exprs: Vec<CExpr> = tangkapan_slot_induk.iter().map(|&s| CExpr::Local(s)).collect();
                Ok(CExpr::FungsiLiteral(nama_sintetis, tangkapan_exprs))
            }
        }
    }
}

// =====================================================================
// 4. NILAI
// =====================================================================

#[derive(Debug, Clone)]
pub enum Value {
    Angka(i64), Desimal(f64), Teks(Rc<str>), Bool(bool),
    Daftar(Rc<Vec<Value>>),
    /// Representasi FLAT untuk daftar yang homogen berisi Angka murni (tanpa campuran tipe).
    /// Dibuat otomatis saat literal daftar (mis. `[1, 2, 3]`) dievaluasi dan semua elemennya
    /// Value::Angka -- lihat `promosikan_daftar_jika_homogen()`. Manfaatnya: 4x lebih hemat
    /// memori dibanding Vec<Value> (8 byte/elemen vs 32 byte/elemen), dan operasi numerik murni
    /// (jumlah/rata_rata) jadi ~9-10x lebih cepat karena compiler bisa auto-vectorize (SIMD)
    /// loop penjumlahan flat i64 -- yang mustahil dilakukan compiler saat elemen masih terbungkus
    /// tag enum. Operasi umum (indexing, gabung, cetak, dst) tetap benar lewat fallback
    /// materialisasi ke Vec<Value> biasa -- lihat `daftar_materialisasi()`.
    DaftarAngka(Rc<Vec<i64>>),
    /// Sama seperti DaftarAngka tapi untuk Desimal murni. Auto-vectorization untuk penjumlahan
    /// float TIDAK sekuat integer (compiler konservatif soal urutan penjumlahan float demi
    /// presisi IEEE-754), jadi speedup di sini lebih kecil (~1.5-2x) dibanding DaftarAngka
    /// (~9-10x) -- tapi penghematan memori (4x) tetap berlaku sama.
    DaftarDesimal(Rc<Vec<f64>>),
    // Kunci Peta/Instans pakai Rc<str> (BUKAN String) SENGAJA -- nama field itu KONSTAN dari
    // sumber program (mis. {"nama": ..., "lahan": ...} atau bentuk Petani{nama:..}), jadi
    // cuma perlu di-alokasi SEKALI saat kompilasi (lihat Instr::MakePeta/BuatInstans -- disimpan
    // sebagai Vec<Rc<str>> di bytecode-nya sendiri). Tiap kali literal ini dieksekusi (mis. di
    // dalam loop 500rb kali, lihat benchmarks/head_to_head/README.md), clone kuncinya jadi
    // Rc::clone (cuma refcount++, bukan heap-alloc+memcpy String baru tiap kali). Ini penyebab
    // #2 dari benchmark yang tercatat di README tsb -- validasi_petani 31x lebih lambat dari
    // Node.js sebelum perbaikan ini.
    Peta(Rc<Vec<(Rc<str>, Value)>>), Kosong,
    /// Instans dari sebuah 'bentuk': nama bentuk + pasangan (field, nilai) sesuai urutan skema.
    /// Representasinya mirip Peta (immutable, clone-on-write) supaya konsisten dengan sisa
    /// bahasa -- tapi field-nya sudah tervalidasi lengkap sejak konstruksi (lihat resolver).
    Instans(Rc<str>, Rc<Vec<(Rc<str>, Value)>>),
    /// Nilai fungsi (dihasilkan literal 'fungsi(...) {...}') -- bisa disimpan di variabel,
    /// dilewatkan sebagai argumen, dst. `idx` menunjuk ke VMFungsi terkompilasi di Pustaka::fungsi,
    /// `tangkapan` adalah snapshot NILAI (bukan referensi hidup) variabel yang ditangkap dari
    /// scope pembungkus saat closure ini dibuat -- lihat komentar resolve_fungsi_umum().
    Fungsi(Rc<NilaiFungsi>),
}

/// Kalau `items` semuanya Value::Angka (tidak kosong), kembalikan Rc<Vec<i64>> flat-nya.
/// Kalau semuanya Value::Desimal, kembalikan varian Desimal. Kalau campuran/kosong/tipe lain,
/// None -- caller lalu pakai Value::Daftar biasa (jalur umum, tidak berubah).
fn coba_promosikan_flat(items: &[Value]) -> Option<Value> {
    if items.is_empty() { return None; }
    if items.iter().all(|v| matches!(v, Value::Angka(_))) {
        let flat: Vec<i64> = items.iter().map(|v| match v { Value::Angka(n) => *n, _ => unreachable!() }).collect();
        return Some(Value::DaftarAngka(Rc::new(flat)));
    }
    if items.iter().all(|v| matches!(v, Value::Desimal(_))) {
        let flat: Vec<f64> = items.iter().map(|v| match v { Value::Desimal(x) => *x, _ => unreachable!() }).collect();
        return Some(Value::DaftarDesimal(Rc::new(flat)));
    }
    None
}

/// Bungkus Vec<Value> jadi Value::Daftar ATAU representasi flat kalau homogen -- dipakai di
/// semua tempat yang tadinya langsung `Value::Daftar(Rc::new(items))`.
fn buat_daftar(items: Vec<Value>) -> Value {
    if let Some(flat) = coba_promosikan_flat(&items) { return flat; }
    Value::Daftar(Rc::new(items))
}

/// Fallback untuk operasi yang belum punya jalur cepat sendiri untuk representasi flat:
/// ubah DaftarAngka/DaftarDesimal balik jadi Vec<Value> biasa (alokasi baru, "jalur lambat"
/// yang cuma dipakai operasi non-numerik seperti indexing/gabung/cetak -- BUKAN jumlah/rata_rata
/// yang punya jalur cepat native sendiri).
fn daftar_materialisasi(v: &Value) -> Option<Rc<Vec<Value>>> {
    match v {
        Value::Daftar(d) => Some(d.clone()),
        Value::DaftarAngka(d) => Some(Rc::new(d.iter().map(|n| Value::Angka(*n)).collect())),
        Value::DaftarDesimal(d) => Some(Rc::new(d.iter().map(|x| Value::Desimal(*x)).collect())),
        _ => None,
    }
}

#[derive(Debug)]
pub struct NilaiFungsi { idx: usize, tangkapan: Vec<Value> }

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Angka(n) => write!(f, "{}", n),
            Value::Desimal(x) => if x.fract() == 0.0 && x.is_finite() { write!(f, "{:.1}", x) } else { write!(f, "{}", x) },
            Value::Teks(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", if *b { "benar" } else { "salah" }),
            Value::Daftar(items) => {
                write!(f, "[")?;
                for (i, v) in items.iter().enumerate() { if i > 0 { write!(f, ", ")?; } write!(f, "{}", v)?; }
                write!(f, "]")
            }
            Value::DaftarAngka(items) => {
                write!(f, "[")?;
                for (i, n) in items.iter().enumerate() { if i > 0 { write!(f, ", ")?; } write!(f, "{}", n)?; }
                write!(f, "]")
            }
            Value::DaftarDesimal(items) => {
                write!(f, "[")?;
                for (i, x) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    if x.fract() == 0.0 && x.is_finite() { write!(f, "{:.1}", x)?; } else { write!(f, "{}", x)?; }
                }
                write!(f, "]")
            }
            Value::Peta(entries) => {
                write!(f, "{{")?;
                for (i, (k, v)) in entries.iter().enumerate() { if i > 0 { write!(f, ", ")?; } write!(f, "\"{}\": {}", k, v)?; }
                write!(f, "}}")
            }
            Value::Kosong => write!(f, "kosong"),
            Value::Instans(nama, entries) => {
                write!(f, "{} {{", nama)?;
                for (i, (k, v)) in entries.iter().enumerate() { if i > 0 { write!(f, ", ")?; } write!(f, "{}: {}", k, v)?; }
                write!(f, "}}")
            }
            Value::Fungsi(_) => write!(f, "<fungsi>"),
        }
    }
}

impl Value {
    fn truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Angka(n) => *n != 0,
            Value::Desimal(x) => *x != 0.0,
            Value::Teks(s) => !s.is_empty(),
            Value::Daftar(d) => !d.is_empty(),
            Value::DaftarAngka(d) => !d.is_empty(),
            Value::DaftarDesimal(d) => !d.is_empty(),
            Value::Peta(p) => !p.is_empty(),
            Value::Kosong => false,
            Value::Instans(..) => true,
            Value::Fungsi(..) => true,
        }
    }
}

fn ke_desimal(v: &Value) -> Option<f64> { match v { Value::Angka(n) => Some(*n as f64), Value::Desimal(f) => Some(*f), _ => None } }

fn nilai_sama(l: &Value, r: &Value) -> bool {
    match (l, r) {
        (Value::Angka(a), Value::Angka(b)) => a == b,
        (Value::Desimal(a), Value::Desimal(b)) => a == b,
        (Value::Angka(a), Value::Desimal(b)) | (Value::Desimal(b), Value::Angka(a)) => (*a as f64) == *b,
        (Value::Teks(a), Value::Teks(b)) => a == b,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::DaftarAngka(a), Value::DaftarAngka(b)) => a == b,
        (Value::DaftarDesimal(a), Value::DaftarDesimal(b)) => a == b,
        (Value::Daftar(a), Value::Daftar(b)) => a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| nilai_sama(x, y)),
        // Kombinasi campuran representasi (mis. DaftarAngka vs Daftar biasa) -- materialisasi
        // dulu baru bandingkan elemen-per-elemen, supaya representasi internal tidak pernah
        // mempengaruhi hasil perbandingan `==` yang terlihat user.
        (l, r) if daftar_materialisasi(l).is_some() && daftar_materialisasi(r).is_some() => {
            let a = daftar_materialisasi(l).unwrap();
            let b = daftar_materialisasi(r).unwrap();
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| nilai_sama(x, y))
        }
        (Value::Peta(a), Value::Peta(b)) => a.len() == b.len() && a.iter().all(|(k, v)| b.iter().any(|(k2, v2)| k == k2 && nilai_sama(v, v2))),
        (Value::Kosong, Value::Kosong) => true,
        (Value::Instans(na, a), Value::Instans(nb, b)) => na == nb && a.len() == b.len() && a.iter().zip(b.iter()).all(|((ka, va), (kb, vb))| ka == kb && nilai_sama(va, vb)),
        _ => false,
    }
}

fn bandingkan(l: Value, r: Value, f: impl Fn(f64, f64) -> bool) -> Result<Value, String> {
    match (ke_desimal(&l), ke_desimal(&r)) {
        (Some(a), Some(b)) => Ok(Value::Bool(f(a, b))),
        _ => Err(format!("Perbandingan hanya berlaku untuk Angka, ditemukan {} dan {}", l, r)),
    }
}

fn eval_binop(l: Value, op: BinOp, r: Value) -> Result<Value, String> {
    use BinOp::*;
    match op {
        Tambah => match (&l, &r) {
            (Value::Teks(_), _) | (_, Value::Teks(_)) => Ok(Value::Teks(format!("{}{}", l, r).into())),
            (Value::Angka(a), Value::Angka(b)) => a.checked_add(*b).map(Value::Angka).ok_or_else(|| format!("Angka meluap (overflow): {} + {} melebihi jangkauan Angka (-9223372036854775808..9223372036854775807). Pertimbangkan pakai Desimal kalau nilainya memang bisa sebesar ini.", a, b)),
            (Value::Angka(_), Value::Desimal(_)) | (Value::Desimal(_), Value::Angka(_)) | (Value::Desimal(_), Value::Desimal(_)) => {
                Ok(Value::Desimal(ke_desimal(&l).unwrap() + ke_desimal(&r).unwrap()))
            }
            _ => Err(format!("Tidak bisa menjumlahkan {} dengan {}", l, r)),
        },
        Kurang => match (&l, &r) {
            (Value::Angka(a), Value::Angka(b)) => a.checked_sub(*b).map(Value::Angka).ok_or_else(|| format!("Angka meluap (overflow): {} - {} melebihi jangkauan Angka (-9223372036854775808..9223372036854775807). Pertimbangkan pakai Desimal kalau nilainya memang bisa sebesar ini.", a, b)),
            _ => match (ke_desimal(&l), ke_desimal(&r)) {
                (Some(a), Some(b)) => Ok(Value::Desimal(a - b)),
                _ => Err(format!("Operator '-' hanya berlaku untuk Angka, ditemukan {} dan {}", l, r)),
            },
        },
        Kali => match (&l, &r) {
            (Value::Angka(a), Value::Angka(b)) => a.checked_mul(*b).map(Value::Angka).ok_or_else(|| format!("Angka meluap (overflow): {} * {} melebihi jangkauan Angka (-9223372036854775808..9223372036854775807). Pertimbangkan pakai Desimal kalau nilainya memang bisa sebesar ini.", a, b)),
            _ => match (ke_desimal(&l), ke_desimal(&r)) {
                (Some(a), Some(b)) => Ok(Value::Desimal(a * b)),
                _ => Err(format!("Operator '*' hanya berlaku untuk Angka, ditemukan {} dan {}", l, r)),
            },
        },
        Bagi => match (&l, &r) {
            (Value::Angka(_), Value::Angka(0)) => Err("Tidak bisa membagi dengan nol.".to_string()),
            // i64::MIN / -1 SECARA MATEMATIS overflow (hasilnya di luar jangkauan i64) --
            // operator '/' Rust polos bisa PANIC (crash) di kasus ini kalau tidak dicek
            // eksplisit (beda dari pembagi-nol biasa, ini murni soal jangkauan angka).
            // Diperlakukan KONSISTEN dengan overflow aritmatika lain (Tambah/Kurang/Kali) --
            // pesan error yang sama, BUKAN crash diam-diam.
            (Value::Angka(a), Value::Angka(b)) => a.checked_div(*b).map(Value::Angka).ok_or_else(|| format!("Angka meluap (overflow): {} / {} melebihi jangkauan Angka (-9223372036854775808..9223372036854775807). Pertimbangkan pakai Desimal kalau nilainya memang bisa sebesar ini.", a, b)),
            _ => match (ke_desimal(&l), ke_desimal(&r)) {
                (Some(_), Some(b)) if b == 0.0 => Err("Tidak bisa membagi dengan nol.".to_string()),
                (Some(a), Some(b)) => Ok(Value::Desimal(a / b)),
                _ => Err(format!("Operator '/' hanya berlaku untuk Angka, ditemukan {} dan {}", l, r)),
            },
        },
        Modulo => match (&l, &r) {
            (Value::Angka(_), Value::Angka(0)) => Err("Tidak bisa modulo dengan nol.".to_string()),
            // Sama seperti Bagi di atas -- i64::MIN % -1 juga overflow secara matematis
            // (Rust '%' polos bisa panic), walau hasil sisa-baginya sendiri well-defined (0).
            // checked_rem() balikin None utk KEDUA kasus (pembagi 0 ATAU overflow ini) --
            // tapi baris di atas sudah nangkep pembagi-nol duluan, jadi None yang sampai sini
            // PASTI kasus overflow MIN/-1 -- aman kembalikan 0 (jawaban matematis yang benar,
          // BUKAN kondisi error, beda dari pembagi-nol).
            (Value::Angka(a), Value::Angka(b)) => Ok(Value::Angka(a.checked_rem(*b).unwrap_or(0))),
            _ => match (ke_desimal(&l), ke_desimal(&r)) {
                (Some(_), Some(b)) if b == 0.0 => Err("Tidak bisa modulo dengan nol.".to_string()),
                (Some(a), Some(b)) => Ok(Value::Desimal(a % b)),
                _ => Err(format!("Operator '%' hanya berlaku untuk Angka, ditemukan {} dan {}", l, r)),
            },
        },
        SamaDengan => Ok(Value::Bool(nilai_sama(&l, &r))),
        TidakSama => Ok(Value::Bool(!nilai_sama(&l, &r))),
        LebihBesar => bandingkan(l, r, |a, b| a > b),
        LebihBesarSama => bandingkan(l, r, |a, b| a >= b),
        LebihKecil => bandingkan(l, r, |a, b| a < b),
        LebihKecilSama => bandingkan(l, r, |a, b| a <= b),
        Dan => Ok(Value::Bool(l.truthy() && r.truthy())),
        Atau => Ok(Value::Bool(l.truthy() || r.truthy())),
    }
}

fn indeks_value(t: Value, i: Value) -> Result<Value, String> {
    match (t, i) {
        (Value::Daftar(d), Value::Angka(n)) => {
            if n < 0 { return Err(format!("Indeks tidak boleh negatif: {}", n)); }
            d.get(n as usize).cloned().ok_or_else(|| format!("Indeks {} di luar jangkauan (panjang daftar: {})", n, d.len()))
        }
        (Value::DaftarAngka(d), Value::Angka(n)) => {
            if n < 0 { return Err(format!("Indeks tidak boleh negatif: {}", n)); }
            d.get(n as usize).map(|x| Value::Angka(*x)).ok_or_else(|| format!("Indeks {} di luar jangkauan (panjang daftar: {})", n, d.len()))
        }
        (Value::DaftarDesimal(d), Value::Angka(n)) => {
            if n < 0 { return Err(format!("Indeks tidak boleh negatif: {}", n)); }
            d.get(n as usize).map(|x| Value::Desimal(*x)).ok_or_else(|| format!("Indeks {} di luar jangkauan (panjang daftar: {})", n, d.len()))
        }
        (Value::Peta(entries), Value::Teks(k)) => {
            entries.iter().find(|(kk, _)| kk.as_ref() == k.as_ref()).map(|(_, v)| v.clone()).ok_or_else(|| format!("Kunci \"{}\" tidak ditemukan di Peta.", k))
        }
        (t, i) => Err(format!("Tidak bisa mengindeks {} dengan {}", t, i)),
    }
}

/// Tulis satu elemen Daftar/Peta -- versi "tulis" dari indeks_value, immutable/clone-on-write
/// (sama seperti SetField/Instans). Dipakai Instr::SetIndeks. Lihat catatan lengkap di
/// definisi Instr::SetIndeks soal semantik Peta (insert-or-update) vs Daftar (harus in-bound).
fn set_indeks_value(t: Value, i: Value, nilai: Value) -> Result<Value, String> {
    match (t, i) {
        (Value::Daftar(d), Value::Angka(n)) => {
            if n < 0 { return Err(format!("Indeks tidak boleh negatif: {}", n)); }
            let idx = n as usize;
            if idx >= d.len() {
                return Err(format!("Indeks {} di luar jangkauan (panjang daftar: {}) -- tidak bisa mengubah elemen yang belum ada, pakai fungsi bawaan 'tambah()' buat menambah elemen baru.", n, d.len()));
            }
            let mut baru = (*d).clone();
            baru[idx] = nilai;
            Ok(Value::Daftar(Rc::new(baru)))
        }
        (Value::DaftarAngka(d), Value::Angka(n)) => {
            if n < 0 { return Err(format!("Indeks tidak boleh negatif: {}", n)); }
            let idx = n as usize;
            if idx >= d.len() {
                return Err(format!("Indeks {} di luar jangkauan (panjang daftar: {}) -- tidak bisa mengubah elemen yang belum ada, pakai fungsi bawaan 'tambah()' buat menambah elemen baru.", n, d.len()));
            }
            // Tetap flat kalau nilai barunya juga Angka (kasus paling umum) -- turun ke Daftar
            // biasa hanya kalau nilai barunya tipe lain (Desimal/Teks/dst), supaya tetap benar.
            if let Value::Angka(baru_n) = nilai {
                let mut baru = (*d).clone();
                baru[idx] = baru_n;
                Ok(Value::DaftarAngka(Rc::new(baru)))
            } else {
                let mut baru: Vec<Value> = d.iter().map(|x| Value::Angka(*x)).collect();
                baru[idx] = nilai;
                Ok(Value::Daftar(Rc::new(baru)))
            }
        }
        (Value::DaftarDesimal(d), Value::Angka(n)) => {
            if n < 0 { return Err(format!("Indeks tidak boleh negatif: {}", n)); }
            let idx = n as usize;
            if idx >= d.len() {
                return Err(format!("Indeks {} di luar jangkauan (panjang daftar: {}) -- tidak bisa mengubah elemen yang belum ada, pakai fungsi bawaan 'tambah()' buat menambah elemen baru.", n, d.len()));
            }
            if let Value::Desimal(baru_x) = nilai {
                let mut baru = (*d).clone();
                baru[idx] = baru_x;
                Ok(Value::DaftarDesimal(Rc::new(baru)))
            } else {
                let mut baru: Vec<Value> = d.iter().map(|x| Value::Desimal(*x)).collect();
                baru[idx] = nilai;
                Ok(Value::Daftar(Rc::new(baru)))
            }
        }
        (Value::Peta(entries), Value::Teks(k)) => {
            let mut baru = (*entries).clone();
            match baru.iter_mut().find(|(kk, _)| kk.as_ref() == k.as_ref()) {
                Some(slot) => slot.1 = nilai,
                None => baru.push((Rc::from(k.as_ref()), nilai)), // kunci baru -> insert (bukan error), konsisten dgn ekspektasi peta dinamis
            }
            Ok(Value::Peta(Rc::new(baru)))
        }
        (t, i) => Err(format!("Tidak bisa mengubah indeks [{}] pada nilai {} (bukan Daftar dgn indeks Angka atau Peta dgn kunci Teks).", i, t)),
    }
}

// =====================================================================
// 4b. ISOTERI IR: optimizer
// =====================================================================
//
// CStmt/CExpr (dihasilkan Resolver di atas) SUDAH berfungsi sebagai lapisan IR Isoteri:
// backend bytecode (Compiler, bagian 5) dan backend JIT (JitEngine, bagian 5b) SAMA-SAMA
// membaca representasi ini, bukan AST mentah (Stmt/Expr) lagi -- itulah yang membuatnya
// "IR" (intermediate representation) dan bukan cuma AST biasa: sudah diresolve (nama
// variabel -> slot lokal/global, field 'bentuk' -> urutan tervalidasi), backend-agnostic,
// dan sekarang jadi SATU tempat optimisasi ditulis yang otomatis menguntungkan SEMUA
// backend (termasuk backend web/ekspor-web) sekaligus -- lihat docs/IR.md.
//
// v1 ini sengaja MINIMAL & konservatif (2 optimisasi paling aman & bernilai tinggi):
//   1. Constant folding: `2 + 3 * 4` -> `14` di waktu kompilasi, bukan runtime.
//   2. Dead code elimination (bentuk paling sederhana): statement setelah 'kembalikan'
//      di blok yang sama dibuang (tidak akan pernah tereksekusi).
// TIDAK termasuk (didokumentasikan sebagai kerja lanjutan di docs/IR.md):
//   - Dead-branch elimination untuk 'kalau' berkondisi konstan (mis. `kalau (benar) {...}`)
//   - Inlining, escape analysis, vectorization/SIMD -- semuanya BUTUH IR ini ada dulu
//     (lihat catatan di README.md soal kenapa SIMD lama direvert).
//
// Constant folding SENGAJA konservatif soal pembagian: `a / 0` TIDAK dilipat (dibiarkan
// apa adanya) supaya pesan error "Tidak bisa membagi dengan nol." dengan nomor barisnya
// yang benar tetap muncul persis seperti sebelum ada optimizer ini -- optimizer tidak
// boleh mengubah PERILAKU yang teramati, cuma mempercepat jalannya.

/// Optimisasi satu blok statement: lipat tiap statement, lalu buang segala sesuatu
/// SETELAH 'kembalikan' pertama (kalau ada) karena pasti tak terjangkau.
fn optimisasi_blok(stmts: Vec<(usize, CStmt)>) -> Vec<(usize, CStmt)> {
    let mut keluar = Vec::with_capacity(stmts.len());
    for (baris, s) in stmts {
        let sudah_kembali = matches!(s, CStmt::Kembalikan(_));
        keluar.push((baris, optimisasi_stmt(s)));
        if sudah_kembali { break; }
    }
    keluar
}

fn optimisasi_jalur(jalur: Vec<CJalur>) -> Vec<CJalur> {
    jalur.into_iter().map(|j| match j {
        CJalur::Field(f) => CJalur::Field(f),
        CJalur::Indeks(e) => CJalur::Indeks(optimisasi_expr(e)),
    }).collect()
}

fn optimisasi_stmt(s: CStmt) -> CStmt {
    match s {
        CStmt::IngatGlobal(slot, e) => CStmt::IngatGlobal(slot, optimisasi_expr(e)),
        CStmt::UbahGlobal(slot, e) => CStmt::UbahGlobal(slot, optimisasi_expr(e)),
        CStmt::IngatLocal(slot, e) => CStmt::IngatLocal(slot, optimisasi_expr(e)),
        CStmt::UbahLocal(slot, e) => CStmt::UbahLocal(slot, optimisasi_expr(e)),
        CStmt::UbahFieldGlobal(slot, path, e) => CStmt::UbahFieldGlobal(slot, path, optimisasi_expr(e)),
        CStmt::UbahFieldLocal(slot, path, e) => CStmt::UbahFieldLocal(slot, path, optimisasi_expr(e)),
        CStmt::UbahJalurGlobal(slot, jalur, e) => CStmt::UbahJalurGlobal(slot, optimisasi_jalur(jalur), optimisasi_expr(e)),
        CStmt::UbahJalurLocal(slot, jalur, e) => CStmt::UbahJalurLocal(slot, optimisasi_jalur(jalur), optimisasi_expr(e)),
        CStmt::Tampilkan(e) => CStmt::Tampilkan(optimisasi_expr(e)),
        CStmt::Kalau(c, tb, eb) => CStmt::Kalau(optimisasi_expr(c), optimisasi_blok(tb), eb.map(optimisasi_blok)),
        CStmt::Ulang(c, b) => CStmt::Ulang(optimisasi_expr(c), optimisasi_blok(b)),
        CStmt::UlangSetiapGlobal(slot, e, b) => CStmt::UlangSetiapGlobal(slot, optimisasi_expr(e), optimisasi_blok(b)),
        CStmt::UlangSetiapLocal(slot, e, b) => CStmt::UlangSetiapLocal(slot, optimisasi_expr(e), optimisasi_blok(b)),
        // Badan 'ulang selaras' TETAP Stmt mentah (evaluator paralel sendiri, lihat bagian 7) --
        // sengaja tidak disentuh optimizer IR ini.
        CStmt::UlangSelaras(e, var, b) => CStmt::UlangSelaras(optimisasi_expr(e), var, b),
        CStmt::CobaGlobal(bc, slot, bt) => CStmt::CobaGlobal(optimisasi_blok(bc), slot, optimisasi_blok(bt)),
        CStmt::CobaLocal(bc, slot, bt) => CStmt::CobaLocal(optimisasi_blok(bc), slot, optimisasi_blok(bt)),
        CStmt::Kembalikan(e) => CStmt::Kembalikan(optimisasi_expr(e)),
        CStmt::EkspresiStmt(e) => CStmt::EkspresiStmt(optimisasi_expr(e)),
        CStmt::Putus => CStmt::Putus,
        CStmt::Lanjut => CStmt::Lanjut,
    }
}

fn optimisasi_expr(e: CExpr) -> CExpr {
    match e {
        CExpr::Binary(l, op, r) => {
            let l = optimisasi_expr(*l);
            let r = optimisasi_expr(*r);
            match lipat_binop(&l, op, &r) {
                Some(hasil) => hasil,
                None => CExpr::Binary(Box::new(l), op, Box::new(r)),
            }
        }
        CExpr::Panggil(nama, args) => CExpr::Panggil(nama, args.into_iter().map(optimisasi_expr).collect()),
        CExpr::Daftar(items) => CExpr::Daftar(items.into_iter().map(optimisasi_expr).collect()),
        CExpr::Peta(entries) => CExpr::Peta(entries.into_iter().map(|(k, v)| (k, optimisasi_expr(v))).collect()),
        CExpr::Indeks(t, i) => CExpr::Indeks(Box::new(optimisasi_expr(*t)), Box::new(optimisasi_expr(*i))),
        CExpr::Tidak(e) => {
            let e = optimisasi_expr(*e);
            match &e {
                CExpr::Bool(b) => CExpr::Bool(!b),
                CExpr::Angka(n) => CExpr::Bool(*n == 0),
                CExpr::Teks(s) => CExpr::Bool(s.is_empty()),
                _ => CExpr::Tidak(Box::new(e)),
            }
        }
        CExpr::Field(t, f) => CExpr::Field(Box::new(optimisasi_expr(*t)), f),
        CExpr::BentukLiteral(nama, entries) => CExpr::BentukLiteral(nama, entries.into_iter().map(|(k, v)| (k, optimisasi_expr(v))).collect()),
        CExpr::FungsiLiteral(nama, tangkapan) => CExpr::FungsiLiteral(nama, tangkapan.into_iter().map(optimisasi_expr).collect()),
        CExpr::PanggilNilai(f, args) => CExpr::PanggilNilai(Box::new(optimisasi_expr(*f)), args.into_iter().map(optimisasi_expr).collect()),
        CExpr::SimpanLaluField(e, slot, f) => CExpr::SimpanLaluField(Box::new(optimisasi_expr(*e)), slot, f),
        lain @ (CExpr::Angka(_) | CExpr::Desimal(_) | CExpr::Teks(_) | CExpr::Bool(_) | CExpr::Global(_) | CExpr::Local(_)) => lain,
    }
}

/// Coba lipat `l op r` jadi satu literal kalau dua-duanya sudah literal SEKARANG (setelah
/// anak-anaknya sendiri dilipat lebih dulu -- lihat optimisasi_expr, jadi ini otomatis
/// menangani ekspresi bersarang seperti `(2 + 3) * 4` lewat rekursi biasa).
/// Pakai checked_* (BUKAN wrapping_*) buat Angka: kalau overflow, JANGAN dilipat -- biarkan
/// CExpr::Binary aslinya diteruskan apa adanya ke runtime, yang sekarang (lihat eval_binop)
/// akan melempar error "Angka meluap" yang jelas, lengkap dengan info baris. Ini bikin
/// perilaku overflow KONSISTEN baik ekspresi konstan (`9223372036854775807 + 1`) maupun hasil
/// runtime (`x + 1` di mana x kebetulan segede itu) -- sama-sama error, bukan salah satunya
/// diam-diam wrap dan satunya error.
fn lipat_binop(l: &CExpr, op: BinOp, r: &CExpr) -> Option<CExpr> {
    use BinOp::*;
    match (l, r) {
        (CExpr::Angka(a), CExpr::Angka(b)) => match op {
            Tambah => a.checked_add(*b).map(CExpr::Angka),
            Kurang => a.checked_sub(*b).map(CExpr::Angka),
            Kali => a.checked_mul(*b).map(CExpr::Angka),
            // checked_div/checked_rem (BUKAN cek "b != 0" doang + operator polos) -- i64::MIN /
            // -1 (dan i64::MIN % -1) overflow secara matematis, operator Rust polos bisa PANIC
            // (CRASH COMPILER-nya sendiri, bukan cuma runtime program user) kalau kebetulan
            // muncul dari rangkaian ekspresi konstan yang di-lipat jadi i64::MIN. Utk Bagi:
            // None (termasuk kasus overflow ini) -> JANGAN dilipat, biarkan runtime yang lempar
            // error jelas (checked_div None juga mencakup b==0, jadi masih konsisten). Utk
            // Modulo: b==0 tetap None (JANGAN dilipat, biarkan runtime lempar error) -- TAPI
            // overflow MIN/-1 aman dilipat jadi 0 langsung (jawaban matematis valid, bukan
            // kondisi error, lihat eval_binop).
            Bagi => a.checked_div(*b).map(CExpr::Angka),
            Modulo => if *b != 0 { Some(CExpr::Angka(a.checked_rem(*b).unwrap_or(0))) } else { None },
            SamaDengan => Some(CExpr::Bool(a == b)),
            TidakSama => Some(CExpr::Bool(a != b)),
            LebihBesar => Some(CExpr::Bool(a > b)),
            LebihBesarSama => Some(CExpr::Bool(a >= b)),
            LebihKecil => Some(CExpr::Bool(a < b)),
            LebihKecilSama => Some(CExpr::Bool(a <= b)),
            Dan => Some(CExpr::Bool(*a != 0 && *b != 0)),
            Atau => Some(CExpr::Bool(*a != 0 || *b != 0)),
        },
        (CExpr::Angka(_) | CExpr::Desimal(_), CExpr::Angka(_) | CExpr::Desimal(_)) => {
            let (Some(a), Some(b)) = (ke_desimal_lit(l), ke_desimal_lit(r)) else { return None };
            match op {
                Tambah => Some(CExpr::Desimal(a + b)),
                Kurang => Some(CExpr::Desimal(a - b)),
                Kali => Some(CExpr::Desimal(a * b)),
                Bagi => if b != 0.0 { Some(CExpr::Desimal(a / b)) } else { None },
                Modulo => if b != 0.0 { Some(CExpr::Desimal(a % b)) } else { None },
                SamaDengan => Some(CExpr::Bool(a == b)),
                TidakSama => Some(CExpr::Bool(a != b)),
                LebihBesar => Some(CExpr::Bool(a > b)),
                LebihBesarSama => Some(CExpr::Bool(a >= b)),
                LebihKecil => Some(CExpr::Bool(a < b)),
                LebihKecilSama => Some(CExpr::Bool(a <= b)),
                Dan => Some(CExpr::Bool(a != 0.0 && b != 0.0)),
                Atau => Some(CExpr::Bool(a != 0.0 || b != 0.0)),
            }
        }
        (CExpr::Teks(a), CExpr::Teks(b)) if matches!(op, Tambah) => Some(CExpr::Teks(format!("{}{}", a, b))),
        (CExpr::Bool(a), CExpr::Bool(b)) => match op {
            Dan => Some(CExpr::Bool(*a && *b)),
            Atau => Some(CExpr::Bool(*a || *b)),
            SamaDengan => Some(CExpr::Bool(a == b)),
            TidakSama => Some(CExpr::Bool(a != b)),
            _ => None,
        },
        _ => None,
    }
}

fn ke_desimal_lit(e: &CExpr) -> Option<f64> {
    match e { CExpr::Angka(n) => Some(*n as f64), CExpr::Desimal(f) => Some(*f), _ => None }
}

// =====================================================================
// 5. BYTECODE: Instr, Compiler
// =====================================================================

#[derive(Debug, Clone)]
enum Instr {
    PushK(usize),
    LoadGlobal(usize), StoreGlobal(usize),
    LoadLocal(usize), StoreLocal(usize),
    BinOp(BinOp),
    Lompat(usize),
    LompatJikaSalah(usize),
    /// Negasi boolean unary -- pop 1 nilai, pakai Value::truthy() (SAMA seperti kondisi
    /// 'kalau'/'dan'/'atau'), push Bool kebalikannya.
    Tidak,
    MakeDaftar(usize),
    MakePeta(Vec<Rc<str>>),
    Indeks,
    /// Sama seperti Indeks (baca container[idx]), TAPI nilai idx-nya TIDAK dibuang -- ditaruh
    /// balik ke stack di bawah hasil baca. Stack sebelum: [..., container, idx]. Stack
    /// sesudah: [..., idx, elemen]. Dipakai KHUSUS oleh Compiler::compile_set_jalur buat
    /// "descend" ke level lebih dalam pas assignment berantai (mis. 'matriks[0][1] = x'),
    /// karena idx level ini masih dibutuhkan lagi belakangan buat Instr::SetIndeks di level
    /// yang sama -- lihat komentar lengkap di compile_set_jalur.
    IndeksTahanIdx,
    /// Tulis satu elemen Daftar/Peta, hasilkan container BARU (immutable/clone-on-write,
    /// konsisten dengan SetField). Stack sebelum: [..., container, idx, nilai_baru]. Stack
    /// sesudah: [..., container_baru]. Peta: kunci yang belum ada otomatis ditambahkan
    /// (insert-or-update); Daftar: indeks harus sudah ada (di luar jangkauan -> error runtime,
    /// TIDAK auto-extend -- pakai fungsi bawaan 'tambah()' buat menambah elemen baru).
    SetIndeks,
    AmbilField(String),
    BuatInstans(Rc<str>, Vec<Rc<str>>),
    SetField(String),
    /// Tambahkan SATU elemen ke Daftar yang disimpan di slot lokal/global, SECARA IN-PLACE
    /// lewat Rc::make_mut kalau memungkinkan -- O(1) amortized, BUKAN clone seluruh isi list
    /// tiap panggilan seperti gabung() generik. Kompiler (compile_stmt & IrLower::lower_stmt,
    /// lihat ekstrak_item_gabung_diri()) memunculkan instruksi ini SEBAGAI GANTI PanggilBawaan
    /// ("gabung",..)+StoreLocal/Global biasa, HANYA saat mengenali persis bentuk assignment
    /// 'x = gabung(x, item)' -- pola SANGAT UMUM buat build list di dalam loop yang sebelumnya
    /// O(n) per panggilan/O(n^2) total (lihat analisis panjang di benchmarks/head_to_head/
    /// README.md -- temuan ini yang memicu optimasi ini). Makna/hasil akhirnya identik dengan
    /// gabung() biasa (termasuk pesan error kalau slotnya bukan Daftar) -- lihat
    /// tambahkan_elemen_inplace(). Stack sebelum: [..., item]. Stack sesudah: kosong (hasil
    /// ditulis langsung ke slot, bukan didorong balik ke stack seperti PanggilBawaan biasa).
    TambahkanLokal(usize),
    TambahkanGlobal(usize),
    /// Duplikasi nilai di puncak stack tanpa mengubahnya -- dipakai buat navigasi rantai
    /// field bersarang (baca "sambil menyimpan" struct perantara supaya bisa di-set balik).
    Dup,
    Tampilkan,
    Pop,
    PanggilFungsi(usize, usize),
    PanggilBawaan(String, usize),
    /// Bikin nilai closure: idx fungsi terkompilasi + berapa nilai tangkapan yang harus dipop
    /// dari puncak stack (sudah didorong compiler sesuai urutan) buat dibungkus jadi Value::Fungsi.
    BuatFungsi(usize, usize),
    /// Panggil NILAI di puncak stack (bukan index fungsi statis) -- dipop dulu argc argumen,
    /// baru nilai callee-nya sendiri, cek itu Value::Fungsi, gabung tangkapan+argumen lalu panggil.
    PanggilNilai(usize),
    IterMulai,
    IterLanjutLocal(usize, usize),
    IterLanjutGlobal(usize, usize),
    JalankanSelaras(String, Vec<(usize, Stmt)>),
    MulaiCobaLocal(usize, usize),
    MulaiCobaGlobal(usize, usize),
    SelesaiCoba,
    /// Tutup PAKSA satu handler 'coba/tangkap' teratas tanpa menjalankan blok 'tangkap'-nya --
    /// dipakai HANYA saat 'putus'/'lanjut' melompat keluar dari dalam blok 'coba' yang masih
    /// aktif di tengah loop (lihat Compiler::coba_depth). Beda dari SelesaiCoba (yang dieksekusi
    /// di akhir jalur normal blok coba): ini dipicu jalur lompatan awal, jadi handler_stack
    /// harus ditutup manual di sini supaya tidak "bocor" ikut aktif ke kode setelah loop.
    TutupHandler,
    TandaiBaris(usize),
    Kembalikan,
}

/// Dua varian pointer fungsi native hasil JIT -- Angka (larik i64 masuk, i64 keluar) atau
/// Desimal (larik f64 masuk, f64 keluar). Keduanya tetap "satu pointer ke larik" per
/// komentar di VMFungsi::native, jadi arity berapa pun tetap satu tipe per mode.
#[derive(Clone, Copy)]
// Varian-variannya cuma pernah dikonstruksi oleh coba_kompilasi_jit()/coba_kompilasi_jit_dari_ir()
// (keduanya di-gate fitur "jit") -- "tidak pernah dikonstruksi" tanpa fitur itu adalah wajar,
// bukan dead code beneran (enum-nya sendiri tetap dipakai penuh di jalur eksekusi lain).
#[cfg_attr(not(feature = "jit"), allow(dead_code))]
enum NativeFn {
    /// (ptr argumen, ptr keluaran flag overflow 1-byte) -> hasil. Pemanggil (lihat
    /// panggil_fungsi_dengan_argumen & Instr::PanggilFungsi) WAJIB baca *ptr_flag setelah
    /// panggilan -- kalau != 0, buang hasilnya & lempar Result::Err "Angka meluap" yang
    /// jelas & catchable lewat 'coba/tangkap', KONSISTEN dengan jalur bytecode biasa
    /// (checked_add dkk di eval BinOp) -- lihat catatan panjang di JitEngine::kompilasi.
    Angka(extern "C" fn(*const i64, *mut i64) -> i64),
    Desimal(extern "C" fn(*const f64) -> f64),
    /// Mode Campur (lihat catatan panjang di enum TipeJit) -- larik argumen berisi CAMPURAN
    /// i64 mentah & bit-pattern f64 (reinterpretasi lewat f64::to_bits(), tersimpan di slot
    /// i64 yang sama, dibaca ulang sesuai tipe SLOT-nya masing-masing oleh kode native --
    /// lihat tipe_reg() di JitEngine::kompilasi_dari_ir). TANPA ptr flag overflow -- mode
    /// Campur SENGAJA tidak boleh aritmatika sama sekali (dicegah cek_jit_murni_nilai), jadi
    /// tidak ada risiko overflow yang perlu dilacak.
    Campur(extern "C" fn(*const i64) -> i64),
}

struct VMFungsi {
    param_count: usize,
    local_slot_count: usize,
    kode: Vec<Instr>,
    /// Terisi kalau fungsi ini berhasil dikompilasi JIT (lihat CFungsi::tipe_jit) --
    /// kalau ada, VM memanggil ini langsung (kode mesin asli) dan sama sekali
    /// melewati bytecode/loop dispatch. Signature-nya SENGAJA dibuat "terima pointer
    /// ke larik" (bukan N parameter langsung) supaya satu tipe fungsi Rust ini
    /// bisa dipakai untuk sembarang jumlah parameter tanpa perlu N varian tipe berbeda --
    /// kode mesin hasil JIT sendiri yang tahu cara baca tiap elemen larik itu.
    native: Option<NativeFn>,
    /// Salinan CFungsi::param_flat, tapi cuma urutan nama field-nya (gak butuh nama bentuk lagi
    /// di runtime) -- dipakai panggil_fungsi_1_arg buat membongkar 1 argumen instans 'bentuk'
    /// jadi beberapa nilai field kalau parameter fungsi ini "flattened", supaya petakan()/
    /// saring()/urutkan() bisa manggil fungsi begini juga (bukan cuma pemanggilan nama statis).
    param_flat: Vec<Option<Vec<String>>>,
    /// Salinan CFungsi::slot_tipe (cuma param_count elemen pertama yang relevan di sini) --
    /// dipakai KHUSUS native==Some(NativeFn::Campur) buat tahu tiap argumen posisi ke-i itu
    /// Angka atau Desimal saat membungkus larik argumen native (lihat
    /// panggil_fungsi_dengan_argumen). Kosong (Vec baru) kalau fungsi ini bukan native Campur
    /// (tidak dipakai, hemat memori -- HampirSemuaFungsi tidak butuh ini).
    slot_tipe: Vec<Option<TipeJit>>,
}

/// Konteks satu loop yang lagi dikompilasi -- dipush saat masuk 'ulang'/'ulang setiap',
/// dipop saat keluar. Ditumpuk (Vec) supaya loop bersarang tetap benar: 'putus'/'lanjut'
/// selalu merujuk loop TERDEKAT (puncak tumpukan).
struct LoopCtx {
    /// Alamat instruksi tujuan 'lanjut' -- sudah pasti diketahui saat body loop mulai
    /// dikompilasi (titik cek kondisi/iterasi), jadi 'lanjut' langsung emit Lompat ke sini
    /// tanpa perlu backpatch.
    continue_target: usize,
    /// Indeks tiap instruksi placeholder Instr::Lompat(0) yang dihasilkan 'putus' -- alamat
    /// aslinya (akhir loop) belum diketahui sampai seluruh body selesai dikompilasi, jadi
    /// di-backpatch belakangan (sama seperti pola if/else/coba yang sudah ada).
    break_patches: Vec<usize>,
    /// Nilai Compiler::coba_depth pada saat loop ini mulai -- dipakai 'putus'/'lanjut' buat
    /// menghitung berapa banyak Instr::TutupHandler perlu disisipkan sebelum lompat (lihat
    /// catatan di Compiler::coba_depth).
    coba_depth_saat_masuk: usize,
}

/// Compiler: mengubah CStmt/CExpr (AST yang sudah di-resolve ke slot) menjadi instruksi bytecode flat.
/// Ini dikerjakan SEKALI di awal (bukan tiap eksekusi), lalu VM tinggal menjalankan array instruksi
/// lewat loop dispatch yang rapat -- jauh lebih cepat dari menyusuri pohon AST berulang-ulang.
struct Compiler {
    konstanta: Vec<Value>, fungsi_index: HashMap<String, usize>,
    loop_stack: Vec<LoopCtx>,
    /// Berapa lapis blok 'coba' aktif yang SEDANG dikompilasi (bukan cuma dilewati) --
    /// dinaikkan tepat sebelum compile_blok(badan_coba), diturunkan tepat sesudahnya (badan
    /// 'tangkap' TIDAK dihitung karena try-nya sudah selesai/handler sudah dipop di titik itu).
    /// Kalau 'putus'/'lanjut' terjadi pas nilai ini lebih besar dari LoopCtx::coba_depth_saat_masuk
    /// loop yang dituju, berarti dia melompat keluar dari tengah 'coba' aktif -- perlu
    /// Instr::TutupHandler sebanyak selisihnya, biar handler_stack VM tidak bocor ikut aktif
    /// ke kode setelah loop (lihat catatan TutupHandler).
    coba_depth: usize,
}

impl Compiler {
    fn new(fungsi_index: HashMap<String, usize>) -> Self { Compiler { konstanta: Vec::new(), fungsi_index, loop_stack: Vec::new(), coba_depth: 0 } }

    fn tambah_konstanta(&mut self, v: Value) -> usize {
        self.konstanta.push(v);
        self.konstanta.len() - 1
    }

    fn compile_top(&mut self, stmts: &[(usize, CStmt)]) -> Vec<Instr> {
        let mut out = Vec::new();
        self.compile_blok(stmts, &mut out);
        out
    }

    fn compile_fungsi(&mut self, f: &CFungsi) -> VMFungsi {
        let mut out = Vec::new();
        self.compile_blok(&f.body, &mut out);
        let param_flat = f.param_flat.iter().map(|p| p.as_ref().map(|(_, field_urut)| field_urut.clone())).collect();
        VMFungsi { param_count: f.param_count, local_slot_count: f.local_slot_count, kode: out, native: None, param_flat, slot_tipe: Vec::new() }
    }

    fn compile_blok(&mut self, stmts: &[(usize, CStmt)], out: &mut Vec<Instr>) {
        for (baris, s) in stmts {
            out.push(Instr::TandaiBaris(*baris));
            self.compile_stmt(s, out);
        }
    }

    /// Diasumsikan puncak stack sudah berisi struct dasar (v0). Menavigasi turun sepanjang
    /// `path[0..path.len()-1]` (baca sambil menyimpan tiap struct perantara lewat Dup, karena
    /// representasi kita immutable/clone-on-write jadi butuh nilai lama buat "set balik"),
    /// menghitung nilai baru, men-set field terakhir, lalu men-set balik tiap field perantara
    /// dari dalam ke luar. Hasil akhirnya v0' (struct dasar yang sudah terbarui) tertinggal
    /// di puncak stack, siap disimpan balik ke slotnya oleh pemanggil.
    fn compile_ubah_field_path(&mut self, path: &[String], value: &CExpr, out: &mut Vec<Instr>) {
        for f in &path[..path.len() - 1] {
            out.push(Instr::Dup);
            out.push(Instr::AmbilField(f.clone()));
        }
        self.compile_expr(value, out);
        for f in path.iter().rev() {
            out.push(Instr::SetField(f.clone()));
        }
    }

    /// Compile assignment berantai (Field/Indeks campur, mis. 'a.b[0].c = x' atau
    /// 'matriks[0][1] = x') -- PRAKONDISI: nilai container buat level jalur[0] SUDAH ada di
    /// puncak stack sebelum instruksi hasil fungsi ini mulai dijalankan (caller yang push).
    /// POSTKONDISI: puncak stack diganti container level jalur[0] yang BARU (hasil delta
    /// diterapkan) -- tinggi stack di bawahnya tidak berubah. Sifat "cuma ganti puncak" ini
    /// yang bikin pemanggilan rekursif ke level lebih dalam bisa langsung disisipkan begitu
    /// saja di tengah, tanpa perlu tracking offset macam-macam.
    ///
    /// Field, non-leaf (masih ada level di bawahnya):
    ///   [C] --Dup--> [C,C] --AmbilField(f)--> [C,INNER] --rekursi--> [C,NEW_INNER] --SetField(f)--> [NEW_C]
    /// Indeks, non-leaf:
    ///   [C] --Dup--> [C,C] --idx--> [C,C,IDX] --IndeksTahanIdx--> [C,IDX,INNER] --rekursi--> [C,IDX,NEW_INNER] --SetIndeks--> [NEW_C]
    /// (leaf, jalur.len()==1, tinggal compile nilai baru & SetField/SetIndeks langsung)
    fn compile_set_jalur(&mut self, jalur: &[CJalur], value: &CExpr, out: &mut Vec<Instr>) {
        match &jalur[0] {
            CJalur::Field(f) => {
                if jalur.len() == 1 {
                    self.compile_expr(value, out);
                    out.push(Instr::SetField(f.clone()));
                } else {
                    out.push(Instr::Dup);
                    out.push(Instr::AmbilField(f.clone()));
                    self.compile_set_jalur(&jalur[1..], value, out);
                    out.push(Instr::SetField(f.clone()));
                }
            }
            CJalur::Indeks(idx) => {
                if jalur.len() == 1 {
                    self.compile_expr(idx, out);
                    self.compile_expr(value, out);
                    out.push(Instr::SetIndeks);
                } else {
                    out.push(Instr::Dup);
                    self.compile_expr(idx, out);
                    out.push(Instr::IndeksTahanIdx);
                    self.compile_set_jalur(&jalur[1..], value, out);
                    out.push(Instr::SetIndeks);
                }
            }
        }
    }

    fn compile_expr(&mut self, e: &CExpr, out: &mut Vec<Instr>) {
        match e {
            CExpr::Angka(n) => { let k = self.tambah_konstanta(Value::Angka(*n)); out.push(Instr::PushK(k)); }
            CExpr::Desimal(f) => { let k = self.tambah_konstanta(Value::Desimal(*f)); out.push(Instr::PushK(k)); }
            CExpr::Teks(s) => { let k = self.tambah_konstanta(Value::Teks(s.clone().into())); out.push(Instr::PushK(k)); }
            CExpr::Bool(b) => { let k = self.tambah_konstanta(Value::Bool(*b)); out.push(Instr::PushK(k)); }
            CExpr::Global(slot) => out.push(Instr::LoadGlobal(*slot)),
            CExpr::Local(slot) => out.push(Instr::LoadLocal(*slot)),
            CExpr::Binary(l, op, r) => { self.compile_expr(l, out); self.compile_expr(r, out); out.push(Instr::BinOp(*op)); }
            CExpr::Tidak(e) => { self.compile_expr(e, out); out.push(Instr::Tidak); }
            CExpr::Panggil(nama, args) => {
                for a in args { self.compile_expr(a, out); }
                if let Some(&idx) = self.fungsi_index.get(nama) {
                    out.push(Instr::PanggilFungsi(idx, args.len()));
                } else {
                    out.push(Instr::PanggilBawaan(nama.clone(), args.len()));
                }
            }
            CExpr::Daftar(items) => {
                for i in items { self.compile_expr(i, out); }
                out.push(Instr::MakeDaftar(items.len()));
            }
            CExpr::Peta(entries) => {
                // Konversi String -> Rc<str> SEKALI di sini (saat kompilasi, bukan tiap
                // eksekusi) -- lihat catatan panjang di enum Value::Peta soal kenapa ini penting.
                let kunci: Vec<Rc<str>> = entries.iter().map(|(k, _)| Rc::from(k.as_str())).collect();
                for (_, v) in entries { self.compile_expr(v, out); }
                out.push(Instr::MakePeta(kunci));
            }
            CExpr::Indeks(t, i) => { self.compile_expr(t, out); self.compile_expr(i, out); out.push(Instr::Indeks); }
            CExpr::Field(t, f) => { self.compile_expr(t, out); out.push(Instr::AmbilField(f.clone())); }
            CExpr::BentukLiteral(nama, entries) => {
                let field_nama: Vec<Rc<str>> = entries.iter().map(|(k, _)| Rc::from(k.as_str())).collect();
                for (_, v) in entries { self.compile_expr(v, out); }
                out.push(Instr::BuatInstans(Rc::from(nama.as_str()), field_nama));
            }
            CExpr::FungsiLiteral(nama_sintetis, tangkapan_exprs) => {
                for e in tangkapan_exprs { self.compile_expr(e, out); }
                let idx = *self.fungsi_index.get(nama_sintetis)
                    .unwrap_or_else(|| panic!("Closure \"{}\" tidak terdaftar -- ini bug internal resolver.", nama_sintetis));
                out.push(Instr::BuatFungsi(idx, tangkapan_exprs.len()));
            }
            CExpr::PanggilNilai(callee, args) => {
                self.compile_expr(callee, out);
                for a in args { self.compile_expr(a, out); }
                out.push(Instr::PanggilNilai(args.len()));
            }
            CExpr::SimpanLaluField(e, slot, field) => {
                self.compile_expr(e, out);
                out.push(Instr::Dup);
                match slot {
                    SlotSasaran::Lokal(n) => out.push(Instr::StoreLocal(*n)),
                    SlotSasaran::Global(n) => out.push(Instr::StoreGlobal(*n)),
                }
                out.push(Instr::AmbilField(field.clone()));
            }
        }
    }

    fn compile_stmt(&mut self, s: &CStmt, out: &mut Vec<Instr>) {
        match s {
            CStmt::IngatGlobal(slot, e) => { self.compile_expr(e, out); out.push(Instr::StoreGlobal(*slot)); }
            CStmt::UbahGlobal(slot, e) => {
                // Peephole: 'x = gabung(x, item)' -> append in-place O(1) amortized, lihat
                // catatan panjang di ekstrak_item_gabung_diri()/tambahkan_elemen_inplace().
                if let Some(item) = ekstrak_item_gabung_diri(e, SlotSasaran::Global(*slot)) {
                    self.compile_expr(item, out);
                    out.push(Instr::TambahkanGlobal(*slot));
                } else {
                    self.compile_expr(e, out);
                    out.push(Instr::StoreGlobal(*slot));
                }
            }
            CStmt::IngatLocal(slot, e) => { self.compile_expr(e, out); out.push(Instr::StoreLocal(*slot)); }
            CStmt::UbahLocal(slot, e) => {
                if let Some(item) = ekstrak_item_gabung_diri(e, SlotSasaran::Lokal(*slot)) {
                    self.compile_expr(item, out);
                    out.push(Instr::TambahkanLokal(*slot));
                } else {
                    self.compile_expr(e, out);
                    out.push(Instr::StoreLocal(*slot));
                }
            }
            CStmt::UbahFieldGlobal(slot, path, e) => {
                out.push(Instr::LoadGlobal(*slot));
                self.compile_ubah_field_path(path, e, out);
                out.push(Instr::StoreGlobal(*slot));
            }
            CStmt::UbahFieldLocal(slot, path, e) => {
                out.push(Instr::LoadLocal(*slot));
                self.compile_ubah_field_path(path, e, out);
                out.push(Instr::StoreLocal(*slot));
            }
            CStmt::UbahJalurGlobal(slot, jalur, e) => {
                out.push(Instr::LoadGlobal(*slot));
                self.compile_set_jalur(jalur, e, out);
                out.push(Instr::StoreGlobal(*slot));
            }
            CStmt::UbahJalurLocal(slot, jalur, e) => {
                out.push(Instr::LoadLocal(*slot));
                self.compile_set_jalur(jalur, e, out);
                out.push(Instr::StoreLocal(*slot));
            }
            CStmt::Tampilkan(e) => { self.compile_expr(e, out); out.push(Instr::Tampilkan); }
            CStmt::Kalau(cond, tb, eb) => {
                self.compile_expr(cond, out);
                let lompat_salah_idx = out.len();
                out.push(Instr::LompatJikaSalah(0));
                self.compile_blok(tb, out);
                if let Some(eb) = eb {
                    let lompat_akhir_idx = out.len();
                    out.push(Instr::Lompat(0));
                    let else_mulai = out.len();
                    out[lompat_salah_idx] = Instr::LompatJikaSalah(else_mulai);
                    self.compile_blok(eb, out);
                    let akhir = out.len();
                    out[lompat_akhir_idx] = Instr::Lompat(akhir);
                } else {
                    let akhir = out.len();
                    out[lompat_salah_idx] = Instr::LompatJikaSalah(akhir);
                }
            }
            CStmt::Ulang(cond, body) => {
                let mulai = out.len();
                self.compile_expr(cond, out);
                let lompat_salah_idx = out.len();
                out.push(Instr::LompatJikaSalah(0));
                self.loop_stack.push(LoopCtx { continue_target: mulai, break_patches: Vec::new(), coba_depth_saat_masuk: self.coba_depth });
                self.compile_blok(body, out);
                let ctx = self.loop_stack.pop().unwrap();
                out.push(Instr::Lompat(mulai));
                let akhir = out.len();
                out[lompat_salah_idx] = Instr::LompatJikaSalah(akhir);
                for idx in ctx.break_patches { out[idx] = Instr::Lompat(akhir); }
            }
            CStmt::UlangSetiapGlobal(slot, e, body) => {
                self.compile_expr(e, out);
                out.push(Instr::IterMulai);
                let mulai = out.len();
                out.push(Instr::IterLanjutGlobal(*slot, 0));
                self.loop_stack.push(LoopCtx { continue_target: mulai, break_patches: Vec::new(), coba_depth_saat_masuk: self.coba_depth });
                self.compile_blok(body, out);
                let ctx = self.loop_stack.pop().unwrap();
                out.push(Instr::Lompat(mulai));
                let akhir = out.len();
                out[mulai] = Instr::IterLanjutGlobal(*slot, akhir);
                for idx in ctx.break_patches { out[idx] = Instr::Lompat(akhir); }
            }
            CStmt::UlangSetiapLocal(slot, e, body) => {
                self.compile_expr(e, out);
                out.push(Instr::IterMulai);
                let mulai = out.len();
                out.push(Instr::IterLanjutLocal(*slot, 0));
                self.loop_stack.push(LoopCtx { continue_target: mulai, break_patches: Vec::new(), coba_depth_saat_masuk: self.coba_depth });
                self.compile_blok(body, out);
                let ctx = self.loop_stack.pop().unwrap();
                out.push(Instr::Lompat(mulai));
                let akhir = out.len();
                out[mulai] = Instr::IterLanjutLocal(*slot, akhir);
                for idx in ctx.break_patches { out[idx] = Instr::Lompat(akhir); }
            }
            CStmt::UlangSelaras(e, var, body) => {
                self.compile_expr(e, out);
                out.push(Instr::JalankanSelaras(var.clone(), body.clone()));
            }
            CStmt::CobaGlobal(badan_coba, slot, badan_tangkap) => {
                let mulai_idx = out.len();
                out.push(Instr::MulaiCobaGlobal(0, *slot));
                self.coba_depth += 1;
                self.compile_blok(badan_coba, out);
                self.coba_depth -= 1;
                out.push(Instr::SelesaiCoba);
                let lompat_akhir_idx = out.len();
                out.push(Instr::Lompat(0));
                let target_tangkap = out.len();
                if let Instr::MulaiCobaGlobal(t, _) = &mut out[mulai_idx] { *t = target_tangkap; }
                self.compile_blok(badan_tangkap, out);
                let akhir = out.len();
                out[lompat_akhir_idx] = Instr::Lompat(akhir);
            }
            CStmt::CobaLocal(badan_coba, slot, badan_tangkap) => {
                let mulai_idx = out.len();
                out.push(Instr::MulaiCobaLocal(0, *slot));
                self.coba_depth += 1;
                self.compile_blok(badan_coba, out);
                self.coba_depth -= 1;
                out.push(Instr::SelesaiCoba);
                let lompat_akhir_idx = out.len();
                out.push(Instr::Lompat(0));
                let target_tangkap = out.len();
                if let Instr::MulaiCobaLocal(t, _) = &mut out[mulai_idx] { *t = target_tangkap; }
                self.compile_blok(badan_tangkap, out);
                let akhir = out.len();
                out[lompat_akhir_idx] = Instr::Lompat(akhir);
            }
            CStmt::Kembalikan(e) => { self.compile_expr(e, out); out.push(Instr::Kembalikan); }
            CStmt::EkspresiStmt(e) => { self.compile_expr(e, out); out.push(Instr::Pop); }
            CStmt::Putus => {
                let ctx = self.loop_stack.last().expect("resolver sudah memvalidasi 'putus' cuma ada di dalam loop");
                for _ in ctx.coba_depth_saat_masuk..self.coba_depth { out.push(Instr::TutupHandler); }
                let idx = out.len();
                out.push(Instr::Lompat(0));
                self.loop_stack.last_mut().unwrap().break_patches.push(idx);
            }
            CStmt::Lanjut => {
                let ctx = self.loop_stack.last().expect("resolver sudah memvalidasi 'lanjut' cuma ada di dalam loop");
                for _ in ctx.coba_depth_saat_masuk..self.coba_depth { out.push(Instr::TutupHandler); }
                out.push(Instr::Lompat(ctx.continue_target));
            }
        }
    }
}

// =====================================================================
// 5b. JIT: kompilasi fungsi "murni" ke kode mesin asli via Cranelift
// =====================================================================
//
// Cranelift cuma ngerti tipe primitif (i64, f64, pointer) -- dia tidak punya
// konsep enum Value dinamis kita. Makanya JIT ini HANYA berlaku untuk fungsi
// yang lolos cek_jit_murni: satu parameter Angka, isinya cuma aritmatika,
// perbandingan, kalau/jika, ulang, dan rekursi ke dirinya sendiri. Fungsi yang
// tidak lolos tetap jalan normal lewat bytecode VM (Pustaka/eksekusi di atas).

// Seluruh region ini (JitEngine, KompilerBadan, KompilerBadanIr) di-gate fitur "jit" --
// TIDAK dipakai build isoteri-wasm/ (lihat Cargo.toml). Fungsi yang lolos cek_jit_murni_*
// (didefinisikan DI LUAR region ini, jadi tetap dievaluasi walau fitur ini off) tanpa fitur
// "jit" otomatis lari ke bytecode VM biasa -- SAMA PERSIS jalur yang sudah dipakai & teruji
// lewat ISOTERI_NO_JIT=1 (lihat scripts/regresi.sh) -- 100% benar, cuma lebih lambat.
#[cfg(feature = "jit")]
struct JitEngine {
    module: cranelift_jit::JITModule,
}

#[cfg(feature = "jit")]
impl JitEngine {
    fn new() -> Self {
        use cranelift::prelude::Configurable;
        let mut flag_builder = cranelift::prelude::settings::builder();
        flag_builder.set("opt_level", "speed").unwrap();
        let isa = cranelift_native::builder()
            .unwrap()
            .finish(cranelift::prelude::settings::Flags::new(flag_builder))
            .unwrap();
        let builder = cranelift_jit::JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        JitEngine { module: cranelift_jit::JITModule::new(builder) }
    }

    /// Mengompilasi satu CFungsi (yang sudah lolos tipe_jit) menjadi kode mesin asli.
    /// Mengembalikan pointer fungsi native -- SATU parameter pointer (bukan N parameter),
    /// supaya arity berapa pun tetap satu tipe signature yang sama per mode (lihat
    /// komentar di NativeFn/VMFungsi::native). Pointer argumennya sendiri selalu alamat
    /// (I64) apapun mode-nya -- yang berubah cuma tipe nilai yang di-load/dikembalikan.
    fn kompilasi(&mut self, f: &CFungsi, mode: TipeJit) -> Result<*const u8, String> {
        use cranelift::prelude::*;
        use cranelift_module::{Linkage, Module};

        let tipe_cl = match mode { TipeJit::Angka => types::I64, TipeJit::Desimal => types::F64, TipeJit::Campur => unreachable!("mode Campur dicegah masuk jalur legacy ini, lihat coba_kompilasi_jit") };

        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64)); // pointer ke larik argumen
        // Mode Angka SAJA dapat parameter ekstra: pointer keluaran 1 byte (I8) buat "flag
        // overflow" -- lihat catatan panjang di flag_var di bawah kenapa Desimal tidak butuh ini
        // (float overflow ke +-inf, bukan wrap-around diam-diam yang menyesatkan seperti i64).
        if mode == TipeJit::Angka { sig.params.push(AbiParam::new(types::I64)); }
        sig.returns.push(AbiParam::new(tipe_cl));
        let func_id = self.module
            .declare_function(&f.nama, Linkage::Local, &sig)
            .map_err(|e| e.to_string())?;

        let mut ctx = self.module.make_context();
        ctx.func.signature = sig;
        let mut fbcx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut fbcx);

        let entry = builder.create_block();
        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);

        for i in 0..f.local_slot_count {
            builder.declare_var(Variable::new(i), tipe_cl);
        }
        // Baca tiap parameter logis dari larik lewat pointer argumen (elemen ke-i di offset i*8 byte).
        let ptr_arg = builder.block_params(entry)[0];
        for i in 0..f.param_count {
            let nilai = builder.ins().load(tipe_cl, MemFlags::new(), ptr_arg, (i * 8) as i32);
            builder.def_var(Variable::new(i), nilai);
        }
        for i in f.param_count..f.local_slot_count {
            let nol = match mode {
                TipeJit::Angka => builder.ins().iconst(types::I64, 0),
                TipeJit::Desimal => builder.ins().f64const(0.0),
                TipeJit::Campur => unreachable!("mode Campur dicegah masuk jalur legacy ini, lihat coba_kompilasi_jit"),
            };
            builder.def_var(Variable::new(i), nol);
        }

        let local_callee = self.module.declare_func_in_func(func_id, builder.func);

        // --- Overflow-trapping (Angka saja): register Variable KHUSUS (indeks tepat setelah
        // seluruh local slot asli, jadi dijamin tidak bentrok) yang menampung "flag overflow"
        // SEPANJANG eksekusI fungsi ini -- bukan hardware trap (yang bakal SIGILL/crash seluruh
        // proses tanpa peduli 'coba/tangkap' pembungkus, beda dari overflow bytecode VM yang
        // catchable lewat checked_add di eval BinOp), tapi diakumulasi (bor) tiap kali operasi
        // Tambah/Kurang/Kali overflow (via sadd_overflow/ssub_overflow/smul_overflow, bukan
        // iadd/isub/imul polos), TERMASUK overflow yang terjadi di panggilan rekursif (flag
        // dari callee dibaca balik lewat parameter kedua & di-OR ke flag milik caller -- lihat
        // CExpr::Panggil di kompilasi_nilai). Baru DICEK & ditulis ke ptr_keluaran saat fungsi
        // benar-benar 'kembalikan' (lihat CStmt::Kembalikan) -- pemanggil Rust (VM) yang baca
        // ptr ini lalu ubah jadi Result::Err "Angka meluap" yang jelas & catchable, KONSISTEN
        // dengan pesan/perilaku overflow di jalur bytecode biasa (lihat panggil_fungsi_dengan_argumen
        // & Instr::PanggilFungsi).
        let flag_var = if mode == TipeJit::Angka {
            let v = Variable::new(f.local_slot_count);
            builder.declare_var(v, types::I8);
            let nol = builder.ins().iconst(types::I8, 0);
            builder.def_var(v, nol);
            Some(v)
        } else { None };
        let out_ptr = if mode == TipeJit::Angka { Some(builder.block_params(entry)[1]) } else { None };

        let mut kompiler = KompilerBadan { builder, local_callee, mode, flag_var, out_ptr };
        let selesai = kompiler.kompilasi_blok(&f.body);
        if !selesai {
            let nol = match mode {
                TipeJit::Angka => kompiler.builder.ins().iconst(types::I64, 0),
                TipeJit::Desimal => kompiler.builder.ins().f64const(0.0),
                TipeJit::Campur => unreachable!("mode Campur dicegah masuk jalur legacy ini, lihat coba_kompilasi_jit"),
            };
            kompiler.tulis_flag_keluaran();
            kompiler.builder.ins().return_(&[nol]);
        }
        kompiler.builder.seal_all_blocks();
        kompiler.builder.finalize();

        self.module.define_function(func_id, &mut ctx).map_err(|e| e.to_string())?;
        self.module.clear_context(&mut ctx);
        self.module.finalize_definitions().map_err(|e| e.to_string())?;

        Ok(self.module.get_finalized_function(func_id))
    }

    /// Migrasi JIT ke IR linear (docs/IR.md poin 3) -- SAMA PERSIS kontrak & elig-nya dengan
    /// `kompilasi()` di atas (dipakai buat `nama`, `mode`, `param_count`), tapi codegen-nya
    /// menelusuri `&[IrInstr]` (hasil lower_fungsi_ke_ir + ir_ke_instr_dgn_konstanta TIDAK
    /// dipakai di sini -- IR LINEAR mentah, sebelum diturunkan ke bytecode Instr, dipakai
    /// LANGSUNG karena target lompatannya sudah berupa index array yang pas buat dipetakan ke
    /// Cranelift Block, tanpa perlu hitung ulang seperti backend bytecode).
    ///
    /// BUKAN jalur produksi -- dipanggil dari jalur validasi `isoteri via-ir` yang sama seperti
    /// bytecode IR linear, dibandingkan HASIL (bukan cuma "berhasil compile") terhadap JIT
    /// produksi (`kompilasi()`) DAN terhadap bytecode murni, lewat regresi yang sama.
    fn kompilasi_dari_ir(&mut self, nama: &str, ir: &[IrInstr], reg_types: &[IrType], param_count: usize, ambang_temp: usize, mode: TipeJit) -> Result<*const u8, String> {
        use cranelift::prelude::*;
        use cranelift_module::{Linkage, Module};

        let tipe_cl = match mode { TipeJit::Angka => types::I64, TipeJit::Desimal => types::F64, TipeJit::Campur => types::I64 }; // Campur: return WAJIB Angka (lihat cek_jit_murni_stmt CStmt::Kembalikan), jadi I64 aman
        // Baca tipe PER-REGISTER dari reg_types (Angka->I64, Desimal->F64, Bool->I8) -- BUKAN
        // cuma "Bool vs tipe_cl seragam" seperti sebelumnya. reg_types SUDAH diisi benar
        // per-slot sejak lower_fungsi_ke_ir() (lihat tipe_dari_jit()) -- fix ini murni di sisi
        // KONSUMSI-nya. Aman buat fungsi mode Angka/Desimal lama (semua slot-nya memang SAMA
        // tipe -> hasilnya identik dengan tipe_cl seperti sebelumnya), DAN sekarang mendukung
        // mode Campur (field beda tipe) dengan benar. Lihat catatan panjang di enum TipeJit.
        let tipe_reg = |r: usize| -> Type {
            match reg_types.get(r).copied() {
                Some(IrType::Bool) => types::I8,
                Some(IrType::Desimal) => types::F64,
                Some(IrType::Angka) => types::I64,
                _ => tipe_cl, // Dinamis/Teks/di luar jangkauan -- tidak seharusnya kejadian di fungsi JIT-elig, tipe_cl sbg fallback aman
            }
        };

        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        if mode == TipeJit::Angka { sig.params.push(AbiParam::new(types::I64)); }
        sig.returns.push(AbiParam::new(tipe_cl));
        let func_id = self.module
            .declare_function(nama, Linkage::Local, &sig)
            .map_err(|e| e.to_string())?;

        let mut ctx = self.module.make_context();
        ctx.func.signature = sig;
        let mut fbcx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut fbcx);

        let entry = builder.create_block();
        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);

        let total_reg = reg_types.len();
        // Cuma register < ambang_temp (local slot ASLI: parameter + `ingat` lokal) yang butuh
        // mesin Variable Cranelift -- register temporary (>= ambang_temp) dicache langsung
        // sebagai nilai SSA mentah lewat KompilerBadanIr::temp_cache (lihat catatan di sana).
        for i in 0..ambang_temp.min(total_reg) { builder.declare_var(Variable::new(i), tipe_reg(i)); }
        let ptr_arg = builder.block_params(entry)[0];
        for i in 0..param_count {
            // tipe_reg(i) (BUKAN tipe_cl seragam) -- inilah titik INTI dukungan mode Campur:
            // tiap parameter di-load sesuai TIPE SLOT-nya sendiri (I64 utk Angka, F64 utk
            // Desimal), bukan asumsi semua parameter tipe yang sama. Aman buat mode Angka/
            // Desimal murni juga (tipe_reg(i) == tipe_cl buat semua i di sana, sama seperti
            // sebelumnya). Lihat catatan panjang di enum TipeJit & NativeFn::Campur (pemanggil
            // Rust sudah membungkus larik argumen sesuai tipe ini, lihat
            // panggil_fungsi_dengan_argumen & Instr::PanggilFungsi).
            let nilai = builder.ins().load(tipe_reg(i), MemFlags::new(), ptr_arg, (i * 8) as i32);
            builder.def_var(Variable::new(i), nilai);
        }
        for i in param_count..ambang_temp.min(total_reg) {
            let nol = match tipe_reg(i) {
                t if t == types::F64 => builder.ins().f64const(0.0),
                t if t == types::I8 => builder.ins().iconst(types::I8, 0),
                _ => builder.ins().iconst(types::I64, 0),
            };
            builder.def_var(Variable::new(i), nol);
        }

        let local_callee = self.module.declare_func_in_func(func_id, builder.func);

        // Overflow-trapping (Angka saja) -- pola SAMA PERSIS dengan kompilasi() (lihat catatan
        // panjang di sana): flag_var pakai index `total_reg` (dijamin belum dipakai Variable
        // manapun -- register asli cuma 0..total_reg).
        let flag_var = if mode == TipeJit::Angka {
            let v = Variable::new(total_reg);
            builder.declare_var(v, types::I8);
            let nol = builder.ins().iconst(types::I8, 0);
            builder.def_var(v, nol);
            Some(v)
        } else { None };
        let out_ptr = if mode == TipeJit::Angka { Some(builder.block_params(entry)[1]) } else { None };

        // --- Pemetaan basic block: leader = index 0, tiap target lompatan, dan index PERSIS
        // setelah tiap Jump/JumpJikaSalah (jalur fallthrough). Semua block dibuat di awal TAPI
        // BELUM di-seal sampai akhir (sama seperti kompilasi() -- deferred sealing, valid di
        // Cranelift selama semua block terisi sebelum finalize()).
        let n = ir.len();
        let mut leader = vec![false; n + 1];
        leader[0] = true;
        for (idx, instr) in ir.iter().enumerate() {
            match instr {
                IrInstr::Jump(t) => { leader[(*t).min(n)] = true; leader[(idx + 1).min(n)] = true; }
                IrInstr::JumpJikaSalah(_, t) => { leader[(*t).min(n)] = true; leader[(idx + 1).min(n)] = true; }
                _ => {}
            }
        }
        let mut block_of: std::collections::HashMap<usize, Block> = std::collections::HashMap::new();
        block_of.insert(0, entry);
        for idx in 1..=n {
            if leader[idx] { block_of.insert(idx, builder.create_block()); }
        }

        let mut kompiler = KompilerBadanIr { builder, local_callee, mode, tipe_reg_fn: &tipe_reg, block_of: &block_of, ambang_temp, temp_cache: std::collections::HashMap::new(), flag_var, out_ptr };
        let mut terminated = false;
        for (idx, instr) in ir.iter().enumerate() {
            if idx > 0 && leader[idx] {
                if !terminated { kompiler.builder.ins().jump(block_of[&idx], &[]); }
                kompiler.builder.switch_to_block(block_of[&idx]);
                terminated = false;
            }
            terminated = kompiler.kompilasi_instr(instr, idx);
        }
        if !terminated {
            let nol = match mode { TipeJit::Angka => kompiler.builder.ins().iconst(types::I64, 0), TipeJit::Desimal => kompiler.builder.ins().f64const(0.0), TipeJit::Campur => kompiler.builder.ins().iconst(types::I64, 0) };
            kompiler.tulis_flag_keluaran();
            kompiler.builder.ins().return_(&[nol]);
        }
        kompiler.builder.seal_all_blocks();
        kompiler.builder.finalize();

        self.module.define_function(func_id, &mut ctx).map_err(|e| e.to_string())?;
        self.module.clear_context(&mut ctx);
        self.module.finalize_definitions().map_err(|e| e.to_string())?;

        Ok(self.module.get_finalized_function(func_id))
    }
}

/// Codegen buat `kompilasi_dari_ir` -- padanan `KompilerBadan` tapi menelusuri `IrInstr` linear
/// (satu instruksi = satu langkah, TIDAK rekursif seperti KompilerBadan yang menelusuri pohon
/// CExpr) dan pakai `Variable::new(reg)` buat SEMUA register (lokal asli MAUPUN temporary --
/// beda dari KompilerBadan yang cuma punya local_slot_count asli, di sini variabelnya lebih
/// banyak tapi Cranelift menanganinya sama saja).
#[cfg(feature = "jit")]
struct KompilerBadanIr<'a, 'b> {
    builder: cranelift::prelude::FunctionBuilder<'a>,
    local_callee: cranelift::codegen::ir::FuncRef,
    mode: TipeJit,
    tipe_reg_fn: &'b dyn Fn(usize) -> cranelift::prelude::Type,
    block_of: &'b std::collections::HashMap<usize, cranelift::prelude::Block>,
    /// Ambang register temporary (== local_slot_count ASLI fungsi ini, lihat parameter
    /// `ambang_temp` di kompilasi_dari_ir). Register >= ini TIDAK lewat mesin Variable/
    /// def_var/use_var Cranelift sama sekali (lihat catatan di `v`/`set` kenapa).
    ambang_temp: usize,
    /// Cache nilai SSA mentah buat register temporary -- Cranelift IR itu SENDIRI sudah SSA,
    /// jadi menaruh hasil sub-ekspresi ke `Variable` (yang mekanismenya buat menangani
    /// REASSIGNMENT lewat resolusi phi-node otomatis) itu kerja EKSTRA yang tidak perlu buat
    /// nilai yang cuma ditulis SEKALI lalu dibaca SEKALI (dijamin oleh IrLower, lihat "8b").
    /// Ini persis kelas masalah yang sama dengan overhead StoreLocal/LoadLocal berlebih yang
    /// dibereskan register allocation v1 di backend bytecode -- di sini versinya buat Cranelift.
    /// AMAN karena (dibuktikan lewat konstruksi IrLower + regresi 17/17): temp TIDAK PERNAH
    /// dibaca dari block Cranelift yang BEDA dari block tempat ia didefinisikan -- begitu ada
    /// percabangan (`kalau`/kondisi), register yang masih "in-flight" sudah pasti terkonsumsi
    /// duluan oleh instruksi percabangan itu sendiri sebelum block baru dimulai.
    temp_cache: std::collections::HashMap<Reg, cranelift::prelude::Value>,
    /// Sama persis semantiknya dengan KompilerBadan::flag_var/out_ptr -- lihat catatan panjang
    /// di JitEngine::kompilasi.
    flag_var: Option<cranelift::prelude::Variable>,
    out_ptr: Option<cranelift::prelude::Value>,
}

#[cfg(feature = "jit")]
impl<'a, 'b> KompilerBadanIr<'a, 'b> {
    fn tulis_flag_keluaran(&mut self) {
        use cranelift::prelude::*;
        if let (Some(fv), Some(op)) = (self.flag_var, self.out_ptr) {
            let nilai = self.builder.use_var(fv);
            self.builder.ins().store(MemFlags::new(), nilai, op, 0);
        }
    }

    fn gabung_flag(&mut self, of: cranelift::prelude::Value) {
        use cranelift::prelude::InstBuilder;
        let fv = self.flag_var.expect("gabung_flag cuma dipanggil di mode Angka, yang selalu punya flag_var");
        let cur = self.builder.use_var(fv);
        let baru = self.builder.ins().bor(cur, of);
        self.builder.def_var(fv, baru);
    }
    fn v(&mut self, r: Reg) -> cranelift::prelude::Value {
        use cranelift::prelude::*;
        if (r as usize) < self.ambang_temp { self.builder.use_var(Variable::new(r as usize)) }
        else { *self.temp_cache.get(&r).unwrap_or_else(|| panic!("bug internal: register temporary {} dibaca sebelum ditulis", r)) }
    }
    fn set(&mut self, r: Reg, val: cranelift::prelude::Value) {
        use cranelift::prelude::*;
        if (r as usize) < self.ambang_temp { self.builder.def_var(Variable::new(r as usize), val); }
        else { self.temp_cache.insert(r, val); }
    }

    /// Kompilasi SATU instruksi IR linear. Return true kalau instruksi ini MENGAKHIRI block
    /// saat ini (Jump/JumpJikaSalah/Kembalikan -- Cranelift butuh tahu ini supaya fallthrough
    /// ke leader berikutnya tidak dobel-terminate).
    fn kompilasi_instr(&mut self, instr: &IrInstr, idx: usize) -> bool {
        use cranelift::prelude::*;
        match instr {
            IrInstr::TandaiBaris(_) => false,
            IrInstr::Const(dst, c) => {
                let v = match c {
                    // Pakai tipe REGISTER TUJUAN (dst), BUKAN self.mode -- di mode Campur,
                    // literal Angka polos (mis. "kembalikan 1") bisa jadi perlu ditaruh di
                    // register Angka ATAUPUN Desimal tergantung konteks pemakaiannya (lihat
                    // catatan panjang di enum TipeJit & tipe_cexpr()). Aman buat mode Angka/
                    // Desimal murni juga (tipe_reg_fn(dst) selalu sama dgn self.mode di sana).
                    IrConst::Angka(n) => match (self.tipe_reg_fn)(*dst as usize) {
                        t if t == types::F64 => self.builder.ins().f64const(*n as f64),
                        _ => self.builder.ins().iconst(types::I64, *n),
                    },
                    IrConst::Desimal(f) => self.builder.ins().f64const(*f),
                    IrConst::Bool(b) => self.builder.ins().iconst(types::I8, if *b { 1 } else { 0 }),
                    IrConst::Teks(_) => unreachable!("cek_jit_murni_nilai seharusnya sudah menyaring literal Teks"),
                };
                self.set(*dst, v);
                false
            }
            IrInstr::Move(dst, src) => { let v = self.v(*src); self.set(*dst, v); false }
            IrInstr::Tidak(..) => unreachable!("Tidak seharusnya sudah disaring cek_jit_murni_nilai/kondisi (JIT sempit ini cuma buat fungsi Angka/Desimal, gak pernah balikin Bool)"),
            IrInstr::BinOp(dst, op, a, b) => {
                use BinOp::*;
                let av = self.v(*a);
                let bv = self.v(*b);
                let hasil = match op {
                    Tambah | Kurang | Kali => match (self.mode, op) {
                        (TipeJit::Angka, Tambah) => { let (r, of) = self.builder.ins().sadd_overflow(av, bv); self.gabung_flag(of); r }
                        (TipeJit::Angka, Kurang) => { let (r, of) = self.builder.ins().ssub_overflow(av, bv); self.gabung_flag(of); r }
                        (TipeJit::Angka, Kali) => { let (r, of) = self.builder.ins().smul_overflow(av, bv); self.gabung_flag(of); r }
                        (TipeJit::Desimal, Tambah) => self.builder.ins().fadd(av, bv),
                        (TipeJit::Desimal, Kurang) => self.builder.ins().fsub(av, bv),
                        (TipeJit::Desimal, Kali) => self.builder.ins().fmul(av, bv),
                        (TipeJit::Campur, _) => unreachable!("mode Campur tidak boleh aritmatika sama sekali, dicegah cek_jit_murni_nilai -- lihat catatan panjang di enum TipeJit"),
                        _ => unreachable!("Bagi seharusnya sudah disaring cek_jit_murni_nilai"),
                    },
                    // Modulo: KHUSUS mode Angka (dicegah cek_jit_murni_nilai buat mode lain).
                    // DUA bahaya native 'srem' x86 yang harus dijinakkan SEBELUM instruksi itu
                    // benar-benar dieksekusi (Cranelift TIDAK short-circuit -- 'select' di
                    // bawah tetap menghitung KEDUA cabangnya sbg nilai SSA, jadi operand srem
                    // itu SENDIRI wajib sudah aman, bukan cuma hasil akhirnya yang diseleksi):
                    // (1) pembagi nol -- error biasa, sama seperti interpreter ("Tidak bisa
                    //     modulo dengan nol."), dilaporkan lewat flag BIT 1 (nilai 2, terpisah
                    //     dari bit 0 punya overflow) -- lihat pembacaan flag di caller.
                    // (2) i64::MIN % -1 -- overflow di hardware (trap SIGFPE), BUKAN kondisi
                    //     yang Rust laporkan sbg error ('%' non-checked Rust malah bisa panic
                    //     di sini, celah LAMA yang sudah ada duluan di interpreter, bukan yang
                    //     kita buat) -- jawaban matematisnya well-defined (0), jadi native code
                    //     di sini malah LEBIH AMAN dari interpreter: kembalikan 0 diam-diam,
                    //     TANPA set flag error apa pun (bukan kasus yang salah, cuma edge case).
                    Modulo => {
                        let nol = self.builder.ins().iconst(types::I64, 0);
                        let satu = self.builder.ins().iconst(types::I64, 1);
                        let neg_satu = self.builder.ins().iconst(types::I64, -1);
                        let min_i64 = self.builder.ins().iconst(types::I64, i64::MIN);
                        let adalah_nol = self.builder.ins().icmp(IntCC::Equal, bv, nol);
                        let adalah_neg_satu = self.builder.ins().icmp(IntCC::Equal, bv, neg_satu);
                        let adalah_min = self.builder.ins().icmp(IntCC::Equal, av, min_i64);
                        let adalah_edge = self.builder.ins().band(adalah_neg_satu, adalah_min);
                        let harus_hindari = self.builder.ins().bor(adalah_nol, adalah_edge);
                        // Paksa pembagi jadi 1 (aman mutlak, hasil 0) SEBELUM srem dipanggil --
                        // bukan nyeleksi HASIL srem, tapi nyeleksi OPERAND-nya, supaya hardware
                        // srem yang sesungguhnya TIDAK PERNAH menerima kombinasi berbahaya.
                        let bv_aman = self.builder.ins().select(harus_hindari, satu, bv);
                        let mentah = self.builder.ins().srem(av, bv_aman);
                        let hasil_akhir = self.builder.ins().select(harus_hindari, nol, mentah);
                        let dua = self.builder.ins().iconst(types::I8, 2);
                        let nol8 = self.builder.ins().iconst(types::I8, 0);
                        let flag_modulo = self.builder.ins().select(adalah_nol, dua, nol8);
                        self.gabung_flag(flag_modulo);
                        hasil_akhir
                    }
                    Dan => self.builder.ins().band(av, bv),
                    Atau => self.builder.ins().bor(av, bv),
                    // Tipe operand (BUKAN self.mode) yang menentukan icmp vs fcmp -- di mode
                    // Campur, `a`/`b` bisa Angka ATAU Desimal tergantung field-nya (lihat
                    // tipe_cexpr() & cek_jit_murni_kondisi yang SUDAH memverifikasi kedua
                    // operand ini same-type sebelum sampai sini, jadi aman pakai tipe SALAH
                    // SATU operand, tidak perlu cek keduanya lagi). Aman juga buat mode Angka/
                    // Desimal murni (tipe_reg_fn(a) selalu sama dgn self.mode di sana).
                    SamaDengan | TidakSama | LebihBesar | LebihBesarSama | LebihKecil | LebihKecilSama => match (self.tipe_reg_fn)(*a as usize) {
                        t if t == types::F64 => {
                            let cc = match op {
                                SamaDengan => FloatCC::Equal, TidakSama => FloatCC::NotEqual,
                                LebihBesar => FloatCC::GreaterThan, LebihBesarSama => FloatCC::GreaterThanOrEqual,
                                LebihKecil => FloatCC::LessThan, LebihKecilSama => FloatCC::LessThanOrEqual,
                                _ => unreachable!(),
                            };
                            self.builder.ins().fcmp(cc, av, bv)
                        }
                        _ => {
                            let cc = match op {
                                SamaDengan => IntCC::Equal, TidakSama => IntCC::NotEqual,
                                LebihBesar => IntCC::SignedGreaterThan, LebihBesarSama => IntCC::SignedGreaterThanOrEqual,
                                LebihKecil => IntCC::SignedLessThan, LebihKecilSama => IntCC::SignedLessThanOrEqual,
                                _ => unreachable!(),
                            };
                            self.builder.ins().icmp(cc, av, bv)
                        }
                    },
                    Bagi => unreachable!("Bagi seharusnya sudah disaring cek_jit_murni_nilai"),
                };
                self.set(*dst, hasil);
                false
            }
            IrInstr::PanggilFungsi(dst, _idx_fungsi, args) => {
                // Elig-JIT (cek_jit_murni_nilai) menjamin SATU-SATUNYA fungsi yang boleh
                // dipanggil di sini adalah dirinya sendiri (rekursi) -- lihat local_callee.
                let nilai: Vec<Value> = args.iter().map(|a| self.v(*a)).collect();
                let slot = self.builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot, (nilai.len().max(1) * 8) as u32,
                ));
                for (i, val) in nilai.iter().enumerate() { self.builder.ins().stack_store(*val, slot, (i * 8) as i32); }
                let addr = self.builder.ins().stack_addr(types::I64, slot, 0);
                let hasil = if self.mode == TipeJit::Angka {
                    // Rekursi mode Angka: sama seperti KompilerBadan::kompilasi_nilai (lihat
                    // catatan panjang di sana) -- balikin flag overflow dari panggilan ini lewat
                    // slot khusus, OR-kan ke flag_var milik fungsi ini.
                    let flag_slot = self.builder.create_sized_stack_slot(StackSlotData::new(
                        StackSlotKind::ExplicitSlot, 1,
                    ));
                    let flag_addr = self.builder.ins().stack_addr(types::I64, flag_slot, 0);
                    let panggilan = self.builder.ins().call(self.local_callee, &[addr, flag_addr]);
                    let hasil = self.builder.inst_results(panggilan)[0];
                    let of_callee = self.builder.ins().stack_load(types::I8, flag_slot, 0);
                    self.gabung_flag(of_callee);
                    hasil
                } else {
                    let panggilan = self.builder.ins().call(self.local_callee, &[addr]);
                    self.builder.inst_results(panggilan)[0]
                };
                self.set(*dst, hasil);
                false
            }
            IrInstr::Jump(t) => { self.builder.ins().jump(self.block_of[t], &[]); true }
            IrInstr::JumpJikaSalah(r, t) => {
                let c = self.v(*r);
                let lanjut = self.block_of[&(idx + 1)];
                self.builder.ins().brif(c, lanjut, &[], self.block_of[t], &[]);
                true
            }
            IrInstr::Kembalikan(r) => { let v = self.v(*r); self.tulis_flag_keluaran(); self.builder.ins().return_(&[v]); true }
            IrInstr::LoadGlobal(..) | IrInstr::StoreGlobal(..) | IrInstr::MakeDaftar(..) | IrInstr::MakePeta(..)
            | IrInstr::Indeks(..) | IrInstr::AmbilField(..) | IrInstr::BuatInstans(..) | IrInstr::BuatFungsi(..)
            | IrInstr::PanggilBawaan(..) | IrInstr::PanggilNilai(..) | IrInstr::Tampilkan(..) | IrInstr::IterMulai(..)
            | IrInstr::IterLanjut(..) | IrInstr::MulaiCoba(..) | IrInstr::SelesaiCoba | IrInstr::Legacy(..) => {
                unreachable!("cek_jit_murni seharusnya sudah menyaring instruksi ini dari fungsi elig-JIT")
            }
        }
    }
}

#[cfg(feature = "jit")]
struct KompilerBadan<'a> {
    builder: cranelift::prelude::FunctionBuilder<'a>,
    local_callee: cranelift::codegen::ir::FuncRef,
    /// Tipe numerik seragam fungsi ini (Angka=i64 atau Desimal=f64) -- menentukan instruksi
    /// Cranelift apa yang dipakai (iadd vs fadd, icmp vs fcmp, dst).
    mode: TipeJit,
    /// Some(Variable) di mode Angka (akumulator flag overflow, I8, di-OR tiap operasi aritmatika
    /// & tiap panggilan rekursif -- lihat catatan panjang di kompilasi()), None di mode Desimal.
    flag_var: Option<cranelift::prelude::Variable>,
    /// Pointer keluaran (parameter kedua fungsi, mode Angka saja) tempat flag_var ditulis
    /// pas fungsi 'kembalikan' -- None di mode Desimal (tidak ada parameter kedua).
    out_ptr: Option<cranelift::prelude::Value>,
}

#[cfg(feature = "jit")]
impl<'a> KompilerBadan<'a> {
    /// Tulis flag_var (kalau mode Angka) ke out_ptr SEBELUM tiap 'return_' -- dipanggil di
    /// SETIAP titik keluar fungsi (CStmt::Kembalikan & fallthrough di akhir kompilasi()),
    /// supaya pemanggil (Rust/VM) selalu baca flag yang sudah final, bukan cuma sebagian.
    fn tulis_flag_keluaran(&mut self) {
        use cranelift::prelude::*;
        if let (Some(fv), Some(op)) = (self.flag_var, self.out_ptr) {
            let nilai = self.builder.use_var(fv);
            self.builder.ins().store(MemFlags::new(), nilai, op, 0);
        }
    }
    /// Jalankan closure yang hasilkan (Value, Value) dari salah satu instruksi *_overflow
    /// Cranelift (sadd_overflow/ssub_overflow/smul_overflow), OR-kan flag overflow-nya ke
    /// flag_var, balikin cuma nilai hasilnya (mode Angka SELALU punya flag_var -- lihat
    /// kompilasi(), jadi unwrap di sini aman).
    fn aritmatika_cek_overflow(
        &mut self,
        lv: cranelift::prelude::Value,
        rv: cranelift::prelude::Value,
        f: impl FnOnce(&mut cranelift::prelude::FunctionBuilder<'a>, cranelift::prelude::Value, cranelift::prelude::Value) -> (cranelift::prelude::Value, cranelift::prelude::Value),
    ) -> cranelift::prelude::Value {
        let (hasil, of) = f(&mut self.builder, lv, rv);
        self.gabung_flag(of);
        hasil
    }

    /// OR-kan satu nilai flag (I8, 0/1) baru ke akumulator flag_var.
    fn gabung_flag(&mut self, of: cranelift::prelude::Value) {
        use cranelift::prelude::InstBuilder;
        let fv = self.flag_var.expect("gabung_flag cuma dipanggil di mode Angka, yang selalu punya flag_var");
        let cur = self.builder.use_var(fv);
        let baru = self.builder.ins().bor(cur, of);
        self.builder.def_var(fv, baru);
    }

    /// Mengembalikan true kalau blok ini PASTI berakhir dengan 'kembalikan'
    /// (jadi block Cranelift saat ini sudah punya terminator).
    fn kompilasi_blok(&mut self, stmts: &[(usize, CStmt)]) -> bool {
        for (_, s) in stmts {
            if self.kompilasi_stmt(s) { return true; }
        }
        false
    }

    fn kompilasi_stmt(&mut self, s: &CStmt) -> bool {
        use cranelift::prelude::*;
        match s {
            CStmt::IngatLocal(slot, e) | CStmt::UbahLocal(slot, e) => {
                let v = self.kompilasi_nilai(e);
                self.builder.def_var(Variable::new(*slot), v);
                false
            }
            CStmt::Kalau(cond, tb, eb) => {
                let c = self.kompilasi_kondisi(cond);
                let then_blk = self.builder.create_block();
                let else_blk = self.builder.create_block();
                let lanjut_blk = self.builder.create_block();

                self.builder.ins().brif(c, then_blk, &[], else_blk, &[]);

                self.builder.switch_to_block(then_blk);
                let then_selesai = self.kompilasi_blok(tb);
                if !then_selesai { self.builder.ins().jump(lanjut_blk, &[]); }
                self.builder.seal_block(then_blk);

                self.builder.switch_to_block(else_blk);
                let else_selesai = if let Some(eb) = eb { self.kompilasi_blok(eb) } else { false };
                if !else_selesai { self.builder.ins().jump(lanjut_blk, &[]); }
                self.builder.seal_block(else_blk);

                if then_selesai && else_selesai {
                    self.builder.seal_block(lanjut_blk);
                    true
                } else {
                    self.builder.switch_to_block(lanjut_blk);
                    self.builder.seal_block(lanjut_blk);
                    false
                }
            }
            CStmt::Ulang(cond, body) => {
                let cek_blk = self.builder.create_block();
                let badan_blk = self.builder.create_block();
                let selesai_blk = self.builder.create_block();

                self.builder.ins().jump(cek_blk, &[]);
                self.builder.switch_to_block(cek_blk);
                let c = self.kompilasi_kondisi(cond);
                self.builder.ins().brif(c, badan_blk, &[], selesai_blk, &[]);

                self.builder.switch_to_block(badan_blk);
                let badan_selesai = self.kompilasi_blok(body);
                if !badan_selesai { self.builder.ins().jump(cek_blk, &[]); }
                self.builder.seal_block(badan_blk);
                self.builder.seal_block(cek_blk);

                self.builder.switch_to_block(selesai_blk);
                self.builder.seal_block(selesai_blk);
                false
            }
            CStmt::Kembalikan(e) => {
                let v = self.kompilasi_nilai(e);
                self.tulis_flag_keluaran();
                self.builder.ins().return_(&[v]);
                true
            }
            CStmt::EkspresiStmt(e) => { self.kompilasi_nilai(e); false }
            _ => unreachable!("cek_jit_murni_stmt seharusnya sudah menyaring stmt yang tidak didukung"),
        }
    }

    /// Kompilasi ekspresi yang menghasilkan NILAI (I64) -- aritmatika/literal/lokal/rekursi.
    fn kompilasi_nilai(&mut self, e: &CExpr) -> cranelift::prelude::Value {
        use cranelift::prelude::*;
        match e {
            CExpr::Angka(n) => match self.mode {
                // Di mode Desimal, literal integer otomatis dipromosikan jadi konstanta f64
                // (sudah divalidasi boleh oleh cek_jit_murni_nilai).
                TipeJit::Angka => self.builder.ins().iconst(types::I64, *n),
                TipeJit::Desimal => self.builder.ins().f64const(*n as f64),
                TipeJit::Campur => unreachable!("mode Campur dicegah masuk jalur legacy ini, lihat coba_kompilasi_jit"),
            },
            CExpr::Desimal(f) => self.builder.ins().f64const(*f),
            CExpr::Local(slot) => self.builder.use_var(Variable::new(*slot)),
            CExpr::Binary(l, op, r) => {
                let lv = self.kompilasi_nilai(l);
                let rv = self.kompilasi_nilai(r);
                match (self.mode, op) {
                    (TipeJit::Angka, BinOp::Tambah) => self.aritmatika_cek_overflow(lv, rv, |b, x, y| b.ins().sadd_overflow(x, y)),
                    (TipeJit::Angka, BinOp::Kurang) => self.aritmatika_cek_overflow(lv, rv, |b, x, y| b.ins().ssub_overflow(x, y)),
                    (TipeJit::Angka, BinOp::Kali) => self.aritmatika_cek_overflow(lv, rv, |b, x, y| b.ins().smul_overflow(x, y)),
                    (TipeJit::Desimal, BinOp::Tambah) => self.builder.ins().fadd(lv, rv),
                    (TipeJit::Desimal, BinOp::Kurang) => self.builder.ins().fsub(lv, rv),
                    (TipeJit::Desimal, BinOp::Kali) => self.builder.ins().fmul(lv, rv),
                    // Sama persis logikanya dengan jalur IR (KompilerBadanIr, lihat catatan
                    // panjang di sana soal 2 bahaya native srem yang dijinakkan pakai select).
                    (TipeJit::Angka, BinOp::Modulo) => {
                        let nol = self.builder.ins().iconst(types::I64, 0);
                        let satu = self.builder.ins().iconst(types::I64, 1);
                        let neg_satu = self.builder.ins().iconst(types::I64, -1);
                        let min_i64 = self.builder.ins().iconst(types::I64, i64::MIN);
                        let adalah_nol = self.builder.ins().icmp(IntCC::Equal, rv, nol);
                        let adalah_neg_satu = self.builder.ins().icmp(IntCC::Equal, rv, neg_satu);
                        let adalah_min = self.builder.ins().icmp(IntCC::Equal, lv, min_i64);
                        let adalah_edge = self.builder.ins().band(adalah_neg_satu, adalah_min);
                        let harus_hindari = self.builder.ins().bor(adalah_nol, adalah_edge);
                        let rv_aman = self.builder.ins().select(harus_hindari, satu, rv);
                        let mentah = self.builder.ins().srem(lv, rv_aman);
                        let hasil_akhir = self.builder.ins().select(harus_hindari, nol, mentah);
                        let dua = self.builder.ins().iconst(types::I8, 2);
                        let nol8 = self.builder.ins().iconst(types::I8, 0);
                        let flag_modulo = self.builder.ins().select(adalah_nol, dua, nol8);
                        self.gabung_flag(flag_modulo);
                        hasil_akhir
                    }
                    _ => unreachable!("cek_jit_murni_nilai seharusnya sudah menyaring operator ini"),
                }
            }
            CExpr::Panggil(_, args) => {
                let nilai: Vec<Value> = args.iter().map(|a| self.kompilasi_nilai(a)).collect();
                // Fungsi native butuh SATU pointer ke larik argumen (bukan N parameter langsung --
                // lihat komentar di VMFungsi::native), jadi di sini kita alokasikan larik 8-byte di
                // stack frame (cukup buat i64 maupun f64), isi tiap argumen ke situ, lalu kirim alamatnya saja.
                let slot = self.builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    (nilai.len().max(1) * 8) as u32,
                ));
                for (i, v) in nilai.iter().enumerate() {
                    self.builder.ins().stack_store(*v, slot, (i * 8) as i32);
                }
                let addr = self.builder.ins().stack_addr(types::I64, slot, 0);
                if self.mode == TipeJit::Angka {
                    // Rekursi mode Angka: panggilan ini sendiri BISA overflow di suatu tempat di
                    // dalam pemanggilan rekursifnya -- flag itu balik lewat parameter kedua
                    // (pointer ke slot 1-byte KHUSUS panggilan ini), kita baca balik lalu OR-kan
                    // ke flag_var milik fungsi INI, supaya overflow dari rekursi manapun tetap
                    // "nyangkut" sampai ke titik 'kembalikan' paling luar (lihat catatan
                    // panjang di kompilasi()).
                    let flag_slot = self.builder.create_sized_stack_slot(StackSlotData::new(
                        StackSlotKind::ExplicitSlot, 1,
                    ));
                    let flag_addr = self.builder.ins().stack_addr(types::I64, flag_slot, 0);
                    let panggilan = self.builder.ins().call(self.local_callee, &[addr, flag_addr]);
                    let hasil = self.builder.inst_results(panggilan)[0];
                    let of_callee = self.builder.ins().stack_load(types::I8, flag_slot, 0);
                    self.gabung_flag(of_callee);
                    hasil
                } else {
                    let panggilan = self.builder.ins().call(self.local_callee, &[addr]);
                    self.builder.inst_results(panggilan)[0]
                }
            }
            _ => unreachable!("cek_jit_murni_nilai seharusnya sudah menyaring ekspresi ini"),
        }
    }

    /// Kompilasi ekspresi kondisi (dipakai khusus di 'kalau'/'ulang') -- perbandingan & logika.
    fn kompilasi_kondisi(&mut self, e: &CExpr) -> cranelift::prelude::Value {
        use cranelift::prelude::*;
        // Kondisi yang sudah terlipat penuh jadi literal Bool oleh optimizer IR (mis. dari
        // `1 < 2` di kode sumber) -- lihat cek_jit_murni_kondisi. iconst 1 bit cukup karena
        // dipakai sebagai operand band/bor/brif seperti hasil icmp/fcmp biasa.
        if let CExpr::Bool(b) = e {
            return self.builder.ins().iconst(cranelift::prelude::types::I8, if *b { 1 } else { 0 });
        }
        if let CExpr::Binary(l, op, r) = e {
            match op {
                BinOp::Dan => { let a = self.kompilasi_kondisi(l); let b = self.kompilasi_kondisi(r); return self.builder.ins().band(a, b); }
                BinOp::Atau => { let a = self.kompilasi_kondisi(l); let b = self.kompilasi_kondisi(r); return self.builder.ins().bor(a, b); }
                _ => {
                    let lv = self.kompilasi_nilai(l);
                    let rv = self.kompilasi_nilai(r);
                    return match self.mode {
                        TipeJit::Angka => {
                            let cc = match op {
                                BinOp::SamaDengan => IntCC::Equal,
                                BinOp::TidakSama => IntCC::NotEqual,
                                BinOp::LebihBesar => IntCC::SignedGreaterThan,
                                BinOp::LebihBesarSama => IntCC::SignedGreaterThanOrEqual,
                                BinOp::LebihKecil => IntCC::SignedLessThan,
                                BinOp::LebihKecilSama => IntCC::SignedLessThanOrEqual,
                                _ => unreachable!(),
                            };
                            self.builder.ins().icmp(cc, lv, rv)
                        }
                        TipeJit::Desimal => {
                            let cc = match op {
                                BinOp::SamaDengan => FloatCC::Equal,
                                BinOp::TidakSama => FloatCC::NotEqual,
                                BinOp::LebihBesar => FloatCC::GreaterThan,
                                BinOp::LebihBesarSama => FloatCC::GreaterThanOrEqual,
                                BinOp::LebihKecil => FloatCC::LessThan,
                                BinOp::LebihKecilSama => FloatCC::LessThanOrEqual,
                                _ => unreachable!(),
                            };
                            self.builder.ins().fcmp(cc, lv, rv)
                        }
                        TipeJit::Campur => unreachable!("mode Campur dicegah masuk jalur legacy ini, lihat coba_kompilasi_jit"),
                    };
                }
            }
        }
        unreachable!("cek_jit_murni_kondisi seharusnya sudah menyaring ekspresi ini")
    }
}


// biasa (single-thread) secepat mungkin -- tapi Rc tidak boleh dikirim antar-thread.
// Daripada memaksakan Arc ke SELURUH Value (yang terbukti bikin kode single-thread
// ~20% lebih lambat walau tidak menyentuh data yang di-share), badan 'ulang selaras'
// dijalankan lewat tipe nilai & evaluator sendiri yang sengaja sederhana (tanpa
// Rc/Arc sama sekali, cukup String biasa) -- konsekuensinya badan paralel dibatasi:
// hanya boleh aritmatika, teks, 'ingat', 'tampilkan', dan 'kalau'/'jika'. Tidak boleh
// memanggil fungsi lain, mengubah variabel luar, atau menyentuh Daftar/Peta.

fn validasi_tubuh_selaras(stmts: &[(usize, Stmt)]) -> Result<(), String> {
    for (_, s) in stmts {
        match s {
            Stmt::Ingat(_, _, e) => validasi_ekspresi_selaras(e)?,
            Stmt::Tampilkan(e) => validasi_ekspresi_selaras(e)?,
            Stmt::Kalau(c, tb, eb) => {
                validasi_ekspresi_selaras(c)?;
                validasi_tubuh_selaras(tb)?;
                if let Some(eb) = eb { validasi_tubuh_selaras(eb)?; }
            }
            _ => return Err(
                "'ulang selaras' hanya boleh berisi 'ingat', 'tampilkan', dan 'kalau'/'jika' -- \
                 tidak boleh memanggil fungsi lain, mengubah variabel di luar loop, memakai \
                 Daftar/Peta, atau 'coba/tangkap' di dalamnya (supaya aman dijalankan di banyak thread sekaligus)."
                    .to_string()
            ),
        }
    }
    Ok(())
}

fn validasi_ekspresi_selaras(e: &Expr) -> Result<(), String> {
    match e {
        Expr::Angka(_) | Expr::Desimal(_) | Expr::Teks(_) | Expr::Bool(_) | Expr::Ident(_) => Ok(()),
        Expr::Binary(l, _, r) => { validasi_ekspresi_selaras(l)?; validasi_ekspresi_selaras(r) }
        _ => Err(
            "Ekspresi di dalam 'ulang selaras' hanya boleh aritmatika, teks, dan variabel -- \
             tidak boleh memanggil fungsi, memakai Daftar/Peta, atau indeks [ ]."
                .to_string()
        ),
    }
}

#[derive(Debug, Clone)]
enum ValorSelaras { Angka(i64), Desimal(f64), Teks(String), Bool(bool) }

impl fmt::Display for ValorSelaras {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ValorSelaras::Angka(n) => write!(f, "{}", n),
            ValorSelaras::Desimal(x) => if x.fract() == 0.0 && x.is_finite() { write!(f, "{:.1}", x) } else { write!(f, "{}", x) },
            ValorSelaras::Teks(s) => write!(f, "{}", s),
            ValorSelaras::Bool(b) => write!(f, "{}", if *b { "benar" } else { "salah" }),
        }
    }
}

fn truthy_selaras(v: &ValorSelaras) -> bool {
    match v {
        ValorSelaras::Bool(b) => *b,
        ValorSelaras::Angka(n) => *n != 0,
        ValorSelaras::Desimal(x) => *x != 0.0,
        ValorSelaras::Teks(s) => !s.is_empty(),
    }
}

fn ke_desimal_selaras(v: &ValorSelaras) -> Option<f64> {
    match v { ValorSelaras::Angka(n) => Some(*n as f64), ValorSelaras::Desimal(f) => Some(*f), _ => None }
}

fn eval_binop_selaras(l: ValorSelaras, op: BinOp, r: ValorSelaras) -> Result<ValorSelaras, String> {
    use BinOp::*;
    match op {
        Tambah => match (&l, &r) {
            (ValorSelaras::Teks(_), _) | (_, ValorSelaras::Teks(_)) => Ok(ValorSelaras::Teks(format!("{}{}", l, r))),
            (ValorSelaras::Angka(a), ValorSelaras::Angka(b)) => a.checked_add(*b).map(ValorSelaras::Angka).ok_or_else(|| format!("Angka meluap (overflow): {} + {} melebihi jangkauan Angka.", a, b)),
            _ => match (ke_desimal_selaras(&l), ke_desimal_selaras(&r)) {
                (Some(a), Some(b)) => Ok(ValorSelaras::Desimal(a + b)),
                _ => Err(format!("Tidak bisa menjumlahkan {} dengan {}", l, r)),
            },
        },
        Kurang => match (&l, &r) {
            (ValorSelaras::Angka(a), ValorSelaras::Angka(b)) => a.checked_sub(*b).map(ValorSelaras::Angka).ok_or_else(|| format!("Angka meluap (overflow): {} - {} melebihi jangkauan Angka.", a, b)),
            _ => match (ke_desimal_selaras(&l), ke_desimal_selaras(&r)) {
                (Some(a), Some(b)) => Ok(ValorSelaras::Desimal(a - b)),
                _ => Err(format!("Operator '-' hanya untuk Angka, ditemukan {} dan {}", l, r)),
            },
        },
        Kali => match (&l, &r) {
            (ValorSelaras::Angka(a), ValorSelaras::Angka(b)) => a.checked_mul(*b).map(ValorSelaras::Angka).ok_or_else(|| format!("Angka meluap (overflow): {} * {} melebihi jangkauan Angka.", a, b)),
            _ => match (ke_desimal_selaras(&l), ke_desimal_selaras(&r)) {
                (Some(a), Some(b)) => Ok(ValorSelaras::Desimal(a * b)),
                _ => Err(format!("Operator '*' hanya untuk Angka, ditemukan {} dan {}", l, r)),
            },
        },
        Bagi => match (&l, &r) {
            (ValorSelaras::Angka(_), ValorSelaras::Angka(0)) => Err("Tidak bisa membagi dengan nol.".to_string()),
            // Sama seperti eval_binop biasa -- i64::MIN / -1 overflow secara matematis, checked_div
            // menangkapnya (Rust '/' polos bisa panic kalau tidak dicek eksplisit).
            (ValorSelaras::Angka(a), ValorSelaras::Angka(b)) => a.checked_div(*b).map(ValorSelaras::Angka).ok_or_else(|| format!("Angka meluap (overflow): {} / {} melebihi jangkauan Angka.", a, b)),
            _ => match (ke_desimal_selaras(&l), ke_desimal_selaras(&r)) {
                (Some(_), Some(b)) if b == 0.0 => Err("Tidak bisa membagi dengan nol.".to_string()),
                (Some(a), Some(b)) => Ok(ValorSelaras::Desimal(a / b)),
                _ => Err(format!("Operator '/' hanya untuk Angka, ditemukan {} dan {}", l, r)),
            },
        },
        Modulo => match (&l, &r) {
            (ValorSelaras::Angka(_), ValorSelaras::Angka(0)) => Err("Tidak bisa modulo dengan nol.".to_string()),
            // Sama seperti eval_binop biasa -- checked_rem balikin None utk i64::MIN % -1 juga
            // (overflow matematis, bukan pembagi-nol) -- pembagi-nol sudah ditangkap baris di
            // atas, jadi None yang sampai sini pasti kasus MIN/-1, aman kembalikan 0.
            (ValorSelaras::Angka(a), ValorSelaras::Angka(b)) => Ok(ValorSelaras::Angka(a.checked_rem(*b).unwrap_or(0))),
            _ => match (ke_desimal_selaras(&l), ke_desimal_selaras(&r)) {
                (Some(_), Some(b)) if b == 0.0 => Err("Tidak bisa modulo dengan nol.".to_string()),
                (Some(a), Some(b)) => Ok(ValorSelaras::Desimal(a % b)),
                _ => Err(format!("Operator '%' hanya untuk Angka, ditemukan {} dan {}", l, r)),
            },
        },
        SamaDengan | TidakSama | LebihBesar | LebihBesarSama | LebihKecil | LebihKecilSama => {
            let sama = match (&l, &r) {
                (ValorSelaras::Angka(a), ValorSelaras::Angka(b)) => (*a as f64) == (*b as f64),
                (ValorSelaras::Teks(a), ValorSelaras::Teks(b)) => a == b,
                (ValorSelaras::Bool(a), ValorSelaras::Bool(b)) => a == b,
                _ => match (ke_desimal_selaras(&l), ke_desimal_selaras(&r)) { (Some(a), Some(b)) => a == b, _ => false },
            };
            match op {
                SamaDengan => Ok(ValorSelaras::Bool(sama)),
                TidakSama => Ok(ValorSelaras::Bool(!sama)),
                _ => match (ke_desimal_selaras(&l), ke_desimal_selaras(&r)) {
                    (Some(a), Some(b)) => Ok(ValorSelaras::Bool(match op {
                        LebihBesar => a > b, LebihBesarSama => a >= b,
                        LebihKecil => a < b, LebihKecilSama => a <= b, _ => unreachable!(),
                    })),
                    _ => Err(format!("Perbandingan hanya untuk Angka, ditemukan {} dan {}", l, r)),
                },
            }
        }
        Dan => Ok(ValorSelaras::Bool(truthy_selaras(&l) && truthy_selaras(&r))),
        Atau => Ok(ValorSelaras::Bool(truthy_selaras(&l) || truthy_selaras(&r))),
    }
}

fn eval_selaras(e: &Expr, scope: &HashMap<String, ValorSelaras>) -> Result<ValorSelaras, String> {
    match e {
        Expr::Angka(n) => Ok(ValorSelaras::Angka(*n)),
        Expr::Desimal(f) => Ok(ValorSelaras::Desimal(*f)),
        Expr::Teks(s) => Ok(ValorSelaras::Teks(s.clone())),
        Expr::Bool(b) => Ok(ValorSelaras::Bool(*b)),
        Expr::Ident(nama) => scope.get(nama).cloned()
            .ok_or_else(|| format!("Variabel \"{}\" tidak dikenal di dalam 'ulang selaras'.", nama)),
        Expr::Binary(l, op, r) => eval_binop_selaras(eval_selaras(l, scope)?, *op, eval_selaras(r, scope)?),
        _ => Err("Ekspresi ini tidak didukung di dalam 'ulang selaras'.".to_string()),
    }
}

fn eksekusi_selaras(stmts: &[(usize, Stmt)], scope: &mut HashMap<String, ValorSelaras>, keluaran: &mut Vec<String>) -> Result<(), String> {
    for (_, s) in stmts {
        match s {
            Stmt::Ingat(nama, _tipe, e) => { let v = eval_selaras(e, scope)?; scope.insert(nama.clone(), v); }
            Stmt::Tampilkan(e) => { let v = eval_selaras(e, scope)?; keluaran.push(format!("{}", v)); }
            Stmt::Kalau(cond, tb, eb) => {
                let c = eval_selaras(cond, scope)?;
                if truthy_selaras(&c) { eksekusi_selaras(tb, scope, keluaran)?; }
                else if let Some(eb) = eb { eksekusi_selaras(eb, scope, keluaran)?; }
            }
            _ => return Err("Pernyataan ini tidak didukung di dalam 'ulang selaras'.".to_string()),
        }
    }
    Ok(())
}

fn value_ke_selaras(v: &Value) -> Result<ValorSelaras, String> {
    match v {
        Value::Angka(n) => Ok(ValorSelaras::Angka(*n)),
        Value::Desimal(f) => Ok(ValorSelaras::Desimal(*f)),
        Value::Teks(s) => Ok(ValorSelaras::Teks(s.to_string())),
        Value::Bool(b) => Ok(ValorSelaras::Bool(*b)),
        lain => Err(format!(
            "'ulang selaras' hanya mendukung item Angka/Desimal/Teks/Bool di dalam daftar, ditemukan {}", lain
        )),
    }
}

/// Menjalankan badan 'ulang selaras' untuk semua item, dipecah ke beberapa thread
/// (sebanyak core CPU yang tersedia). Hasil 'tampilkan' dikumpulkan per-item lalu
/// dicetak di AKHIR sesuai urutan asli daftar -- supaya output tetap deterministik
/// walau komputasinya berjalan tidak berurutan di banyak core.
fn jalankan_selaras(var: &str, items: Vec<ValorSelaras>, body: &[(usize, Stmt)]) -> Result<Vec<Vec<String>>, String> {
    if items.is_empty() { return Ok(Vec::new()); }

    let n_thread = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1).max(1);
    let chunk_size = (items.len() + n_thread - 1) / n_thread;
    let mut hasil: Vec<Vec<String>> = vec![Vec::new(); items.len()];

    let hasil_per_chunk: Result<Vec<Vec<(usize, Vec<String>)>>, String> = std::thread::scope(|s| {
        let mut handles = Vec::new();
        for (chunk_idx, chunk) in items.chunks(chunk_size).enumerate() {
            let start_idx = chunk_idx * chunk_size;
            let handle = s.spawn(move || -> Result<Vec<(usize, Vec<String>)>, String> {
                let mut lokal = Vec::with_capacity(chunk.len());
                for (i, item) in chunk.iter().enumerate() {
                    let mut scope: HashMap<String, ValorSelaras> = HashMap::new();
                    scope.insert(var.to_string(), item.clone());
                    let mut keluaran = Vec::new();
                    eksekusi_selaras(body, &mut scope, &mut keluaran)?;
                    lokal.push((start_idx + i, keluaran));
                }
                Ok(lokal)
            });
            handles.push(handle);
        }
        handles.into_iter()
            .map(|h| h.join().map_err(|_| "Salah satu thread paralel gagal (panic).".to_string())?)
            .collect()
    });

    for chunk_hasil in hasil_per_chunk? {
        for (idx, keluaran) in chunk_hasil { hasil[idx] = keluaran; }
    }
    Ok(hasil)
}



/// Bagian VM yang read-only setelah kompilasi selesai (konstanta & tabel fungsi).
/// Dipisah dari state yang berubah supaya: (1) tidak perlu di-clone tiap panggilan fungsi
/// -- cukup dipinjam (borrow) biasa, gratis; (2) nantinya bisa dibagi ke banyak thread
/// sekaligus lewat satu Arc<Pustaka>, dasar untuk 'ulang selaras' (paralel).
pub struct Pustaka {
    konstanta: Vec<Value>,
    fungsi: Vec<Rc<VMFungsi>>,
    /// Peta nama fungsi -> indeksnya di `fungsi` -- dipakai buat panggilan-balik (callback)
    /// dari builtin seperti petakan()/saring()/urutkan() yang terima nama fungsi user
    /// sebagai argumen Teks, lalu perlu manggil balik fungsi itu per-item.
    nama_ke_indeks: HashMap<String, usize>,
}

enum SlotVar { Local(usize), Global(usize) }

/// Satu penangan 'coba/tangkap' aktif: kalau error terjadi selagi ini masih di tumpukan,
/// eksekusi dialihkan ke instruksi `target` (awal blok 'tangkap'), dengan operand stack
/// dikembalikan ke `stack_base` dan pesan errornya disimpan ke `slot`.
struct PenangkapError { stack_base: usize, target: usize, slot: SlotVar }

/// Bagian VM yang berubah selama eksekusi (unik per-thread saat nanti dipakai paralel).
struct VMState {
    globals: Vec<Value>,
    stack: Vec<Value>,
    iter_stack: Vec<(Vec<Value>, usize)>,
    locals_stack: Vec<Value>,
    handler_stack: Vec<PenangkapError>,
    baris_sekarang: usize,
}

pub struct VM {
    pustaka: Pustaka,
    state: VMState,
}

impl VM {
    pub fn new(global_slot_count: usize, konstanta: Vec<Value>, fungsi: Vec<Rc<VMFungsi>>, nama_ke_indeks: HashMap<String, usize>) -> Self {
        VM {
            pustaka: Pustaka { konstanta, fungsi, nama_ke_indeks },
            state: VMState {
                globals: vec![Value::Kosong; global_slot_count],
                stack: Vec::with_capacity(256),
                iter_stack: Vec::new(),
                locals_stack: Vec::with_capacity(256),
                handler_stack: Vec::new(),
                baris_sekarang: 0,
            },
        }
    }

    pub fn jalankan_top(&mut self, kode: &[Instr]) -> Result<(), String> {
        eksekusi(&self.pustaka, &mut self.state, kode, 0)?;
        Ok(())
    }
}

/// Tempelkan nomor baris ke pesan error, TAPI cuma sekali -- kalau pesan sudah
/// diawali "Baris N:" (berarti sudah ditempelkan di frame yang lebih dalam saat
/// error itu pertama kali terjadi), biarkan apa adanya supaya tidak dobel.
fn dengan_baris(pesan: String, baris: usize) -> String {
    if pesan.starts_with("Baris ") { pesan } else { format!("Baris {}: {}", baris, pesan) }
}

/// Panggil fungsi user (bytecode ATAU native JIT, dua-duanya didukung) dengan tepat 1 argumen --
/// dipakai sebagai mesin panggilan-balik buat petakan()/saring()/urutkan().
fn panggil_fungsi_1_arg(pustaka: &Pustaka, state: &mut VMState, idx: usize, arg: Value) -> Result<Value, String> {
    let f = &pustaka.fungsi[idx];
    if f.param_flat.len() != 1 {
        return Err(format!("Fungsi callback butuh tepat 1 parameter (item-nya sendiri), tapi fungsi ini punya {} parameter.", f.param_flat.len()));
    }
    let argumen = match &f.param_flat[0] {
        // Parameter callback ini "flattened" (instans 'bentuk' numerik-murni, lihat
        // CFungsi::param_flat) -- bongkar `arg` (harus instans) jadi nilai per-field sesuai
        // urutan skema, supaya petakan()/saring()/urutkan() tetap bisa manggil fungsi begini.
        Some(field_urut) => {
            let entries = match &arg {
                Value::Instans(_, entries) => entries,
                lain => return Err(format!("Fungsi callback ini butuh instans 'bentuk', ditemukan {}", lain)),
            };
            let mut v = Vec::with_capacity(field_urut.len());
            for fnama in field_urut {
                let val = entries.iter().find(|(k, _)| k.as_ref() == fnama.as_str()).map(|(_, val)| val.clone())
                    .ok_or_else(|| format!("Instans tidak punya field \"{}\" yang dibutuhkan fungsi callback.", fnama))?;
                v.push(val);
            }
            v
        }
        None => vec![arg],
    };
    panggil_fungsi_dengan_argumen(pustaka, state, idx, argumen)
}

/// Panggil satu "callback 1-argumen" buat petakan()/saring()/urutkan() -- terima DUA bentuk
/// sebagai argumen kedua: nama fungsi (Teks, cara lama yang tetap didukung persis seperti
/// sebelumnya) ATAU closure first-class (Value::Fungsi -- closure literal inline
/// 'fungsi(x) {...}', closure bernama tersimpan di variabel, atau nama fungsi biasa yang
/// dilewatkan sebagai NILAI tanpa tanda kutip). Kalau closure-nya punya tangkapan (capture),
/// itu otomatis "transparan" buat pemanggil builtin -- yang perlu dipikirkan cuma argumen
/// terakhir (si item daftar), sisanya sudah beres di belakang layar lewat NilaiFungsi::tangkapan.
fn panggil_callback_1_arg(pustaka: &Pustaka, state: &mut VMState, nama_builtin: &str, callback: &Value, arg: Value) -> Result<Value, String> {
    match callback {
        Value::Teks(s) => {
            let idx = *pustaka.nama_ke_indeks.get(s.as_ref())
                .ok_or_else(|| format!("{}(): fungsi \"{}\" tidak ditemukan.", nama_builtin, s))?;
            panggil_fungsi_1_arg(pustaka, state, idx, arg)
        }
        Value::Fungsi(nf) => {
            let f = &pustaka.fungsi[nf.idx];
            let n_tangkapan = nf.tangkapan.len();
            let n_param_asli = f.param_flat.len().saturating_sub(n_tangkapan);
            if n_param_asli != 1 {
                return Err(format!("{}(): closure callback butuh tepat 1 parameter (item-nya sendiri) di luar variabel yang ditangkap, tapi closure ini punya {}.", nama_builtin, n_param_asli));
            }
            // Param TERAKHIR di param_flat yang relevan (bukan slot tangkapan) -- lihat catatan
            // urutan slot di NilaiFungsi & resolve_fungsi_umum ("tangkapan dulu, baru parameter").
            let argumen_asli = match f.param_flat.last().and_then(|x| x.as_ref()) {
                Some(field_urut) => {
                    let entries = match &arg {
                        Value::Instans(_, entries) => entries,
                        lain => return Err(format!("{}(): closure callback ini butuh instans 'bentuk', ditemukan {}", nama_builtin, lain)),
                    };
                    let mut v = Vec::with_capacity(field_urut.len());
                    for fnama in field_urut {
                        let val = entries.iter().find(|(k, _)| k.as_ref() == fnama.as_str()).map(|(_, val)| val.clone())
                            .ok_or_else(|| format!("Instans tidak punya field \"{}\" yang dibutuhkan closure callback.", fnama))?;
                        v.push(val);
                    }
                    v
                }
                None => vec![arg],
            };
            let mut argumen_lengkap = nf.tangkapan.clone();
            argumen_lengkap.extend(argumen_asli);
            panggil_fungsi_dengan_argumen(pustaka, state, nf.idx, argumen_lengkap)
        }
        lain => Err(format!("{}(): argumen kedua harus Teks (nama fungsi) atau closure/fungsi sebagai nilai, ditemukan {}", nama_builtin, lain)),
    }
}


/// Versi umum panggil_fungsi_1_arg buat sembarang jumlah argumen -- dipakai buat memanggil
/// Value::Fungsi (closure) lewat Instr::PanggilNilai. `argumen` sudah termasuk tangkapan
/// closure DI DEPAN (kalau ada), diikuti argumen dari titik pemanggilan, persis sesuai urutan
/// slot lokal fungsi tujuan (lihat resolve_fungsi_umum: tangkapan dulu, baru parameter).
fn panggil_fungsi_dengan_argumen(pustaka: &Pustaka, state: &mut VMState, idx: usize, argumen: Vec<Value>) -> Result<Value, String> {
    let f = &pustaka.fungsi[idx];
    if argumen.len() != f.param_count {
        return Err(format!("Fungsi ini butuh {} argumen, tapi diberi {}.", f.param_count, argumen.len()));
    }
    if let Some(native) = f.native {
        match native {
            NativeFn::Angka(native) => {
                let mut larik = Vec::with_capacity(argumen.len());
                for v in argumen {
                    match v {
                        Value::Angka(n) => larik.push(n),
                        lain => return Err(format!("Argumen untuk fungsi native (JIT) harus Angka, ditemukan {}", lain)),
                    }
                }
                let mut flag: i64 = 0;
                let hasil = native(larik.as_ptr(), &mut flag as *mut i64);
                // Flag encoding (lihat gabung_flag di KompilerBadan/KompilerBadanIr): bit 0
                // (nilai 1) = overflow aritmatika, bit 1 (nilai 2) = modulo dengan nol. Dua-duanya
                // di-OR-kan (bor) selama eksekusi, jadi bisa saja KEDUANYA kejadian sebelum
                // fungsi return (mekanisme ini TIDAK short-circuit di titik kejadian, cuma
                // dicek di akhir) -- prioritaskan overflow (konsisten dgn urutan cek interpreter
                // biasa), tapi tetap tangani modulo-nol sendiri kalau overflow tidak kejadian.
                if flag & 1 != 0 {
                    return Err(format!("Angka meluap (overflow) di dalam fungsi terkompilasi JIT: hasil melebihi jangkauan Angka (-9223372036854775808..9223372036854775807). Pertimbangkan pakai Desimal kalau nilainya memang bisa sebesar ini."));
                }
                if flag & 2 != 0 {
                    return Err("Tidak bisa modulo dengan nol.".to_string());
                }
                Ok(Value::Angka(hasil))
            }
            NativeFn::Desimal(native) => {
                let mut larik = Vec::with_capacity(argumen.len());
                for v in argumen {
                    match v {
                        Value::Desimal(n) => larik.push(n),
                        Value::Angka(n) => larik.push(n as f64),
                        lain => return Err(format!("Argumen untuk fungsi native (JIT) harus Desimal, ditemukan {}", lain)),
                    }
                }
                Ok(Value::Desimal(native(larik.as_ptr())))
            }
            NativeFn::Campur(native) => {
                // Bungkus tiap argumen SESUAI TIPE SLOT-nya masing-masing (f.slot_tipe) --
                // Angka jadi i64 mentah, Desimal jadi bit-pattern f64 (f64::to_bits(), disimpan
                // di slot i64 yang sama -- kode native yang tahu cara baca ulang sesuai tipe
                // ASLI-nya, lihat tipe_reg() di JitEngine::kompilasi_dari_ir). Lihat catatan
                // panjang di enum TipeJit & NativeFn::Campur kenapa ini aman.
                let mut larik: Vec<i64> = Vec::with_capacity(argumen.len());
                for (i, v) in argumen.into_iter().enumerate() {
                    let t = f.slot_tipe.get(i).copied().flatten();
                    match (t, v) {
                        (Some(TipeJit::Angka), Value::Angka(n)) => larik.push(n),
                        (Some(TipeJit::Desimal), Value::Desimal(n)) => larik.push(n.to_bits() as i64),
                        (Some(TipeJit::Desimal), Value::Angka(n)) => larik.push((n as f64).to_bits() as i64),
                        (t, lain) => return Err(format!("Argumen ke-{} untuk fungsi native (JIT) harus {:?}, ditemukan {}", i + 1, t, lain)),
                    }
                }
                Ok(Value::Angka(native(larik.as_ptr())))
            }
        }
    } else {
        let base = state.locals_stack.len();
        state.locals_stack.resize(base + f.local_slot_count, Value::Kosong);
        for (i, v) in argumen.into_iter().enumerate() { state.locals_stack[base + i] = v; }
        let hasil = eksekusi(pustaka, state, &f.kode, base)?.unwrap_or(Value::Kosong);
        state.locals_stack.truncate(base);
        Ok(hasil)
    }
}

/// Bandingkan dua nilai buat kebutuhan urutkan() -- hanya Angka/Desimal (dibandingkan
/// numerik, boleh campur) dan Teks (dibandingkan leksikografis) yang didukung.
fn bandingkan_nilai(a: &Value, b: &Value) -> Result<std::cmp::Ordering, String> {
    match (a, b) {
        (Value::Teks(x), Value::Teks(y)) => Ok(x.cmp(y)),
        _ => match (ke_desimal(a), ke_desimal(b)) {
            (Some(x), Some(y)) => Ok(x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal)),
            _ => Err(format!("urutkan() cuma bisa buat daftar berisi Angka/Desimal atau Teks (gak campur), ditemukan {} dan {}", a, b)),
        },
    }
}

/// Loop dispatch inti VM. Mengembalikan Some(nilai) kalau 'kembalikan' tereksekusi.
/// `pustaka` dipinjam (bukan di-clone) di jalur panggilan fungsi -- karena Arc::clone
/// itu atomic increment (sedikit lebih mahal dari Rc), kalau dilakukan jutaan kali di
/// rekursi berat itu kerasa. Meminjam biasa di sini nol biaya.
///
/// Error TIDAK langsung dipropagasi lewat '?' seperti biasa -- tiap kali eksekusi_satu()
/// gagal, loop di sini cek dulu apakah ada 'coba/tangkap' aktif MILIK frame ini sendiri
/// (handler_stack.len() > base_handler, bukan warisan dari fungsi pemanggil). Kalau ada,
/// program TIDAK berhenti: pesan error disimpan ke variabel 'tangkap' dan eksekusi
/// lanjut dari situ. Kalau tidak ada, error dipropagasi seperti biasa ke pemanggil.
fn eksekusi(pustaka: &Pustaka, state: &mut VMState, kode: &[Instr], locals_base: usize) -> Result<Option<Value>, String> {
    let base_stack = state.stack.len();
    let base_handler = state.handler_stack.len();
    let mut pc = 0usize;

    while pc < kode.len() {
        match eksekusi_satu(pustaka, state, kode, locals_base, &mut pc) {
            Ok(None) => { /* lanjut, pc sudah diupdate di dalam eksekusi_satu */ }
            Ok(Some(v)) => {
                state.stack.truncate(base_stack);
                state.handler_stack.truncate(base_handler);
                return Ok(Some(v));
            }
            Err(pesan) => {
                let pesan = dengan_baris(pesan, state.baris_sekarang);
                if state.handler_stack.len() > base_handler {
                    let handler = state.handler_stack.pop().unwrap();
                    state.stack.truncate(handler.stack_base);
                    match handler.slot {
                        SlotVar::Local(s) => state.locals_stack[locals_base + s] = Value::Teks(pesan.into()),
                        SlotVar::Global(s) => state.globals[s] = Value::Teks(pesan.into()),
                    }
                    pc = handler.target;
                } else {
                    state.stack.truncate(base_stack);
                    state.handler_stack.truncate(base_handler);
                    return Err(pesan);
                }
            }
        }
    }
    state.stack.truncate(base_stack);
    state.handler_stack.truncate(base_handler);
    Ok(None)
}

/// Menjalankan SATU instruksi. Ok(None) = lanjut (pc sudah diupdate di dalam sini),
/// Ok(Some(v)) = 'kembalikan' tereksekusi, Err = ada kesalahan (belum tentu fatal --
/// eksekusi() di atas yang memutuskan mau ditangkap 'coba/tangkap' atau dipropagasi).
fn eksekusi_satu(pustaka: &Pustaka, state: &mut VMState, kode: &[Instr], locals_base: usize, pc: &mut usize) -> Result<Option<Value>, String> {
            match &kode[*pc] {
                Instr::TandaiBaris(baris) => { state.baris_sekarang = *baris; *pc += 1; }
                Instr::PushK(i) => { state.stack.push(pustaka.konstanta[*i].clone()); *pc += 1; }
                Instr::LoadGlobal(s) => { state.stack.push(state.globals[*s].clone()); *pc += 1; }
                Instr::StoreGlobal(s) => { let v = state.stack.pop().unwrap(); state.globals[*s] = v; *pc += 1; }
                Instr::LoadLocal(s) => { state.stack.push(state.locals_stack[locals_base + *s].clone()); *pc += 1; }
                Instr::StoreLocal(s) => { let v = state.stack.pop().unwrap(); state.locals_stack[locals_base + *s] = v; *pc += 1; }
                Instr::TambahkanLokal(s) => {
                    let item = state.stack.pop().unwrap();
                    let lama = std::mem::replace(&mut state.locals_stack[locals_base + *s], Value::Kosong);
                    state.locals_stack[locals_base + *s] = tambahkan_elemen_inplace(lama, item)?;
                    *pc += 1;
                }
                Instr::TambahkanGlobal(s) => {
                    let item = state.stack.pop().unwrap();
                    let lama = std::mem::replace(&mut state.globals[*s], Value::Kosong);
                    state.globals[*s] = tambahkan_elemen_inplace(lama, item)?;
                    *pc += 1;
                }
                Instr::BinOp(op) => {
                    let r = state.stack.pop().unwrap();
                    let l = state.stack.pop().unwrap();
                    state.stack.push(eval_binop(l, *op, r)?);
                    *pc += 1;
                }
                Instr::Lompat(target) => { *pc = *target; }
                Instr::LompatJikaSalah(target) => {
                    let v = state.stack.pop().unwrap();
                    if v.truthy() { *pc += 1; } else { *pc = *target; }
                }
                Instr::Tidak => {
                    let v = state.stack.pop().unwrap();
                    state.stack.push(Value::Bool(!v.truthy()));
                    *pc += 1;
                }
                Instr::MakeDaftar(n) => {
                    let mut items = Vec::with_capacity(*n);
                    for _ in 0..*n { items.push(state.stack.pop().unwrap()); }
                    items.reverse();
                    state.stack.push(buat_daftar(items));
                    *pc += 1;
                }
                Instr::MakePeta(kunci) => {
                    let mut nilai = Vec::with_capacity(kunci.len());
                    for _ in 0..kunci.len() { nilai.push(state.stack.pop().unwrap()); }
                    nilai.reverse();
                    let entries: Vec<(Rc<str>, Value)> = kunci.iter().cloned().zip(nilai.into_iter()).collect();
                    state.stack.push(Value::Peta(entries.into()));
                    *pc += 1;
                }
                Instr::Indeks => {
                    let i = state.stack.pop().unwrap();
                    let t = state.stack.pop().unwrap();
                    state.stack.push(indeks_value(t, i)?);
                    *pc += 1;
                }
                Instr::IndeksTahanIdx => {
                    let i = state.stack.pop().unwrap();
                    let t = state.stack.pop().unwrap();
                    let v = indeks_value(t, i.clone())?;
                    state.stack.push(i);
                    state.stack.push(v);
                    *pc += 1;
                }
                Instr::SetIndeks => {
                    let nilai_baru = state.stack.pop().unwrap();
                    let i = state.stack.pop().unwrap();
                    let t = state.stack.pop().unwrap();
                    state.stack.push(set_indeks_value(t, i, nilai_baru)?);
                    *pc += 1;
                }
                Instr::AmbilField(field) => {
                    let t = state.stack.pop().unwrap();
                    match &t {
                        Value::Instans(nama, entries) => {
                            let v = entries.iter().find(|(k, _)| k.as_ref() == field.as_str()).map(|(_, v)| v.clone())
                                .ok_or_else(|| format!("Bentuk \"{}\" tidak punya field \"{}\".", nama, field))?;
                            state.stack.push(v);
                        }
                        lain => return Err(format!("Akses field \".{}\" hanya berlaku untuk instans 'bentuk', ditemukan {}", field, lain)),
                    }
                    *pc += 1;
                }
                Instr::BuatInstans(nama, field_nama) => {
                    let mut nilai = Vec::with_capacity(field_nama.len());
                    for _ in 0..field_nama.len() { nilai.push(state.stack.pop().unwrap()); }
                    nilai.reverse();
                    let entries: Vec<(Rc<str>, Value)> = field_nama.iter().cloned().zip(nilai.into_iter()).collect();
                    state.stack.push(Value::Instans(nama.clone(), Rc::new(entries)));
                    *pc += 1;
                }
                Instr::SetField(field) => {
                    let baru = state.stack.pop().unwrap();
                    let t = state.stack.pop().unwrap();
                    match t {
                        Value::Instans(nama, entries) => {
                            if !entries.iter().any(|(k, _)| k.as_ref() == field.as_str()) {
                                return Err(format!("Bentuk \"{}\" tidak punya field \"{}\".", nama, field));
                            }
                            let mut baru_entries = (*entries).clone();
                            for (k, v) in baru_entries.iter_mut() { if k.as_ref() == field.as_str() { *v = baru; break; } }
                            state.stack.push(Value::Instans(nama, Rc::new(baru_entries)));
                        }
                        lain => return Err(format!("Tidak bisa mengubah field \".{}\" pada nilai {} (bukan instans 'bentuk').", field, lain)),
                    }
                    *pc += 1;
                }
                Instr::Dup => { let v = state.stack.last().unwrap().clone(); state.stack.push(v); *pc += 1; }
                Instr::BuatFungsi(idx, jumlah_tangkapan) => {
                    let mulai = state.stack.len() - jumlah_tangkapan;
                    let tangkapan: Vec<Value> = state.stack.drain(mulai..).collect();
                    state.stack.push(Value::Fungsi(Rc::new(NilaiFungsi { idx: *idx, tangkapan })));
                    *pc += 1;
                }
                Instr::PanggilNilai(argc) => {
                    let mulai = state.stack.len() - argc;
                    let argumen_panggilan: Vec<Value> = state.stack.drain(mulai..).collect();
                    let callee = state.stack.pop().unwrap();
                    let nf = match callee {
                        Value::Fungsi(nf) => nf,
                        lain => return Err(format!("Nilai ini bukan fungsi, gak bisa dipanggil: {}", lain)),
                    };
                    let mut argumen_lengkap = nf.tangkapan.clone();
                    argumen_lengkap.extend(argumen_panggilan);
                    let hasil = panggil_fungsi_dengan_argumen(pustaka, state, nf.idx, argumen_lengkap)?;
                    state.stack.push(hasil);
                    *pc += 1;
                }
                Instr::Tampilkan => { let v = state.stack.pop().unwrap(); println!("{}", v); *pc += 1; }
                Instr::Pop => { state.stack.pop(); *pc += 1; }
                Instr::PanggilFungsi(idx, argc) => {
                    let f = &pustaka.fungsi[*idx];
                    if f.param_count != *argc {
                        return Err(format!("Fungsi mengharapkan {} argumen, tapi diberikan {}.", f.param_count, argc));
                    }
                    if let Some(native) = f.native {
                        // Jalur JIT: langsung panggil kode mesin asli, lewati bytecode VM sepenuhnya.
                        // Susun argumen jadi larik sesuai mode (i64 atau f64), lalu kirim pointer-nya --
                        // signature native seragam untuk sembarang jumlah parameter per mode (lihat VMFungsi::native).
                        let args_start = state.stack.len() - argc;
                        let hasil = match native {
                            NativeFn::Angka(native) => {
                                let mut larik: Vec<i64> = Vec::with_capacity(*argc);
                                for v in state.stack.drain(args_start..) {
                                    match v {
                                        Value::Angka(n) => larik.push(n),
                                        lain => return Err(format!("Argumen untuk fungsi native (JIT) harus Angka, ditemukan {}", lain)),
                                    }
                                }
                                let mut flag: i64 = 0;
                                let hasil = native(larik.as_ptr(), &mut flag as *mut i64);
                                // Sama seperti pembacaan flag di panggil_fungsi_dengan_argumen
                                // (lihat catatan panjang di sana) -- bit 0 overflow, bit 1
                                // modulo-dengan-nol.
                                if flag & 1 != 0 {
                                    return Err(format!("Angka meluap (overflow) di dalam fungsi terkompilasi JIT: hasil melebihi jangkauan Angka (-9223372036854775808..9223372036854775807). Pertimbangkan pakai Desimal kalau nilainya memang bisa sebesar ini."));
                                }
                                if flag & 2 != 0 {
                                    return Err("Tidak bisa modulo dengan nol.".to_string());
                                }
                                Value::Angka(hasil)
                            }
                            NativeFn::Desimal(native) => {
                                let mut larik: Vec<f64> = Vec::with_capacity(*argc);
                                for v in state.stack.drain(args_start..) {
                                    match v {
                                        Value::Desimal(n) => larik.push(n),
                                        Value::Angka(n) => larik.push(n as f64),
                                        lain => return Err(format!("Argumen untuk fungsi native (JIT) harus Desimal, ditemukan {}", lain)),
                                    }
                                }
                                Value::Desimal(native(larik.as_ptr()))
                            }
                            NativeFn::Campur(native) => {
                                // Sama seperti NativeFn::Campur di panggil_fungsi_dengan_argumen
                                // (lihat catatan panjang di sana) -- bungkus tiap argumen sesuai
                                // tipe slot-nya (f.slot_tipe), Angka jadi i64 mentah, Desimal jadi
                                // bit-pattern f64.
                                let mut larik: Vec<i64> = Vec::with_capacity(*argc);
                                for (i, v) in state.stack.drain(args_start..).enumerate() {
                                    let t = f.slot_tipe.get(i).copied().flatten();
                                    match (t, v) {
                                        (Some(TipeJit::Angka), Value::Angka(n)) => larik.push(n),
                                        (Some(TipeJit::Desimal), Value::Desimal(n)) => larik.push(n.to_bits() as i64),
                                        (Some(TipeJit::Desimal), Value::Angka(n)) => larik.push((n as f64).to_bits() as i64),
                                        (t, lain) => return Err(format!("Argumen ke-{} untuk fungsi native (JIT) harus {:?}, ditemukan {}", i + 1, t, lain)),
                                    }
                                }
                                Value::Angka(native(larik.as_ptr()))
                            }
                        };
                        state.stack.push(hasil);
                    } else {
                        let base = state.locals_stack.len();
                        state.locals_stack.resize(base + f.local_slot_count, Value::Kosong);
                        let args_start = state.stack.len() - argc;
                        for i in 0..*argc {
                            state.locals_stack[base + i] = std::mem::replace(&mut state.stack[args_start + i], Value::Kosong);
                        }
                        state.stack.truncate(args_start);
                        let hasil = eksekusi(pustaka, state, &f.kode, base)?.unwrap_or(Value::Kosong);
                        state.locals_stack.truncate(base);
                        state.stack.push(hasil);
                    }
                    *pc += 1;
                }
                Instr::PanggilBawaan(nama, argc) => {
                    let args_start = state.stack.len() - argc;
                    let hasil = match nama.as_str() {
                        "petakan" | "saring" if *argc == 2 => {
                            let callback = state.stack[args_start + 1].clone();
                            let daftar = match daftar_materialisasi(&state.stack[args_start]) {
                                Some(d) => (*d).clone(),
                                None => return Err(format!("{}(daftar, fungsi): argumen pertama harus Daftar, ditemukan {}", nama, state.stack[args_start])),
                            };
                            state.stack.truncate(args_start);
                            if nama == "petakan" {
                                let mut hasil_daftar = Vec::with_capacity(daftar.len());
                                for item in daftar { hasil_daftar.push(panggil_callback_1_arg(pustaka, state, "petakan", &callback, item)?); }
                                buat_daftar(hasil_daftar)
                            } else {
                                let mut hasil_daftar = Vec::with_capacity(daftar.len());
                                for item in daftar {
                                    match panggil_callback_1_arg(pustaka, state, "saring", &callback, item.clone())? {
                                        Value::Bool(true) => hasil_daftar.push(item),
                                        Value::Bool(false) => {}
                                        lain => return Err(format!("saring(): fungsi penyaring harus mengembalikan Bool, ditemukan {}", lain)),
                                    }
                                }
                                buat_daftar(hasil_daftar)
                            }
                        }
                        "urutkan" if *argc == 1 || *argc == 2 => {
                            let callback = if *argc == 2 { Some(state.stack[args_start + 1].clone()) } else { None };
                            let daftar = match daftar_materialisasi(&state.stack[args_start]) {
                                Some(d) => (*d).clone(),
                                None => return Err(format!("urutkan(): argumen pertama harus Daftar, ditemukan {}", state.stack[args_start])),
                            };
                            state.stack.truncate(args_start);
                            let hasil_daftar = if let Some(cb) = callback {
                                let mut berkunci = Vec::with_capacity(daftar.len());
                                for item in daftar { let kunci = panggil_callback_1_arg(pustaka, state, "urutkan", &cb, item.clone())?; berkunci.push((kunci, item)); }
                                let mut kesalahan = None;
                                berkunci.sort_by(|(ka, _), (kb, _)| bandingkan_nilai(ka, kb).unwrap_or_else(|e| { kesalahan = Some(e); std::cmp::Ordering::Equal }));
                                if let Some(e) = kesalahan { return Err(e); }
                                berkunci.into_iter().map(|(_, v)| v).collect()
                            } else {
                                let mut d = daftar;
                                let mut kesalahan = None;
                                d.sort_by(|a, b| bandingkan_nilai(a, b).unwrap_or_else(|e| { kesalahan = Some(e); std::cmp::Ordering::Equal }));
                                if let Some(e) = kesalahan { return Err(e); }
                                d
                            };
                            buat_daftar(hasil_daftar)
                        }
                        "server_mulai" if *argc == 2 => {
                            let port = match &state.stack[args_start] {
                                Value::Angka(n) => *n,
                                lain => return Err(format!("server_mulai(port, handler): argumen pertama (port) harus Angka, ditemukan {}", lain)),
                            };
                            if !(1..=65535).contains(&port) {
                                return Err(format!("server_mulai(): port harus 1-65535, ditemukan {}", port));
                            }
                            let handler = state.stack[args_start + 1].clone();
                            state.stack.truncate(args_start);
                            jalankan_http_server(pustaka, state, port as u16, &handler)?;
                            Value::Kosong
                        }
                        _ => {
                            let hasil = panggil_bawaan(nama, &state.stack[args_start..])?
                                .ok_or_else(|| format!("Fungsi \"{}\" tidak ditemukan.", nama))?;
                            state.stack.truncate(args_start);
                            hasil
                        }
                    };
                    state.stack.push(hasil);
                    *pc += 1;
                }
                Instr::IterMulai => {
                    let v = state.stack.pop().unwrap();
                    match daftar_materialisasi(&v) {
                        Some(items) => state.iter_stack.push(((*items).clone(), 0)),
                        None => return Err(format!("'ulang setiap' butuh Daftar, ditemukan {}", v)),
                    }
                    *pc += 1;
                }
                Instr::IterLanjutLocal(slot, target) => {
                    let selesai = {
                        let (items, pos) = state.iter_stack.last_mut().unwrap();
                        if *pos < items.len() { state.locals_stack[locals_base + *slot] = items[*pos].clone(); *pos += 1; false } else { true }
                    };
                    if selesai { state.iter_stack.pop(); *pc = *target; } else { *pc += 1; }
                }
                Instr::IterLanjutGlobal(slot, target) => {
                    let selesai = {
                        let (items, pos) = state.iter_stack.last_mut().unwrap();
                        if *pos < items.len() { state.globals[*slot] = items[*pos].clone(); *pos += 1; false } else { true }
                    };
                    if selesai { state.iter_stack.pop(); *pc = *target; } else { *pc += 1; }
                }
                Instr::JalankanSelaras(var, body) => {
                    let daftar_val = state.stack.pop().unwrap();
                    let items: Vec<ValorSelaras> = match daftar_materialisasi(&daftar_val) {
                        Some(d) => d.iter().map(value_ke_selaras).collect::<Result<Vec<_>, _>>()?,
                        None => return Err(format!("'ulang selaras' butuh Daftar, ditemukan {}", daftar_val)),
                    };
                    let hasil = jalankan_selaras(var, items, body)?;
                    for keluaran in hasil {
                        for baris in keluaran { println!("{}", baris); }
                    }
                    *pc += 1;
                }
                Instr::MulaiCobaLocal(target, slot) => {
                    state.handler_stack.push(PenangkapError { stack_base: state.stack.len(), target: *target, slot: SlotVar::Local(*slot) });
                    *pc += 1;
                }
                Instr::MulaiCobaGlobal(target, slot) => {
                    state.handler_stack.push(PenangkapError { stack_base: state.stack.len(), target: *target, slot: SlotVar::Global(*slot) });
                    *pc += 1;
                }
                Instr::SelesaiCoba => {
                    state.handler_stack.pop(); // blok 'coba' selesai TANPA error, buang penangannya
                    *pc += 1;
                }
                Instr::TutupHandler => {
                    // Sama seperti SelesaiCoba (pop satu handler), tapi dipicu 'putus'/'lanjut'
                    // yang melompat keluar tengah blok 'coba' -- lihat catatan di definisi Instr.
                    state.handler_stack.pop();
                    *pc += 1;
                }
                Instr::Kembalikan => {
                    let v = state.stack.pop().unwrap();
                    return Ok(Some(v));
                }
            }
    Ok(None)
}

/// Tambahkan `item` ke `val` (harus salah satu varian Daftar) SECARA IN-PLACE lewat
/// Rc::make_mut kalau memungkinkan (refcount Rc == 1 -- SATU-SATUNYA pemilik saat ini, yaitu
/// `val` sendiri yang baru diambil-alih dari slot lewat mem::replace, lihat Instr::TambahkanLokal/
/// Global di eksekusi_satu). Kalau Rc SEDANG dibagi ke pemilik lain (mis. ada 'ingat y = x' di
/// baris sebelumnya, y masih pegang versi lama) -- Rc::make_mut otomatis clone dulu SEBELUM
/// mutasi, correctness immutability tetap terjamin, cuma jalur lambatnya sama seperti gabung()
/// lama. Inilah yang membuat pola SANGAT UMUM 'x = gabung(x, item)' di dalam loop jadi O(1)
/// amortized (bukan O(n) per panggilan/O(n^2) total) TANPA mengorbankan semantik immutable
/// Isoteri di kasus lain (mis. 'y = x' lalu 'x = gabung(x, item)' -- y tetap versi lama, tetap
/// benar) -- lihat analisis lengkap & angka before/after di benchmarks/head_to_head/README.md.
fn tambahkan_elemen_inplace(val: Value, item: Value) -> Result<Value, String> {
    match val {
        Value::Daftar(mut d) => { Rc::make_mut(&mut d).push(item); Ok(Value::Daftar(d)) }
        Value::DaftarAngka(mut d) => match item {
            Value::Angka(n) => { Rc::make_mut(&mut d).push(n); Ok(Value::DaftarAngka(d)) }
            lain => {
                // Item bukan Angka -- demosi ke Daftar umum (jalur langka, sama-sama lebih
                // lambat seperti gabung() lama untuk kasus campuran tipe, correctness tetap
                // terjaga -- cuma jalur cepatnya yang tidak berlaku di sini).
                let mut baru: Vec<Value> = d.iter().map(|n| Value::Angka(*n)).collect();
                baru.push(lain);
                Ok(Value::Daftar(Rc::new(baru)))
            }
        },
        Value::DaftarDesimal(mut d) => match item {
            Value::Desimal(x) => { Rc::make_mut(&mut d).push(x); Ok(Value::DaftarDesimal(d)) }
            lain => {
                let mut baru: Vec<Value> = d.iter().map(|x| Value::Desimal(*x)).collect();
                baru.push(lain);
                Ok(Value::Daftar(Rc::new(baru)))
            }
        },
        // Pesan error SENGAJA sama persis kata-katanya dengan gabung() biasa (lihat
        // panggil_bawaan match "gabung") -- dari sudut pandang pengguna Isoteri, ini efeknya
        // harus 100% tak terbedakan dari 'x = gabung(x, item)' biasa, cuma lebih cepat.
        lain => Err(format!("gabung() argumen pertama harus Daftar, ditemukan {}", lain)),
    }
}

/// Kalau `e` persis berbentuk 'gabung(<slot yang sama dengan target>, item)' -- pola SANGAT
/// UMUM 'x = gabung(x, item)' di dalam loop -- kembalikan referensi ke ekspresi `item`-nya
/// supaya caller (Compiler::compile_stmt & IrLower::lower_stmt) bisa mengganti jalur
/// StoreLocal/Global generik dengan Instr::TambahkanLokal/Global yang O(1) amortized (lihat
/// tambahkan_elemen_inplace()). None kalau bentuknya bukan pola ini PERSIS -- caller tetap
/// pakai jalur gabung() generik lama, correctness tidak pernah dikorbankan demi kecepatan
/// (mis. 'x = gabung(y, item)' atau 'x = gabung(x, item) + 1' TIDAK cocok, sengaja jatuh ke
/// jalur lambat lama -- optimasi ini SEMPIT & konservatif, bukan analisis alias umum).
fn ekstrak_item_gabung_diri(e: &CExpr, target: SlotSasaran) -> Option<&CExpr> {
    let CExpr::Panggil(nama, args) = e else { return None };
    if nama != "gabung" || args.len() != 2 { return None; }
    let cocok = match (&args[0], target) {
        (CExpr::Local(s), SlotSasaran::Lokal(t)) => *s == t,
        (CExpr::Global(s), SlotSasaran::Global(t)) => *s == t,
        _ => false,
    };
    if cocok { Some(&args[1]) } else { None }
}

/// Standard Library dasar: daftar, peta/JSON, berkas, jaringan.
fn panggil_bawaan(nama: &str, args: &[Value]) -> Result<Option<Value>, String> {
    match nama {
        "panjang" => match args.get(0) {
            Some(Value::Daftar(d)) => Ok(Some(Value::Angka(d.len() as i64))),
            Some(Value::DaftarAngka(d)) => Ok(Some(Value::Angka(d.len() as i64))),
            Some(Value::DaftarDesimal(d)) => Ok(Some(Value::Angka(d.len() as i64))),
            Some(Value::Teks(s)) => Ok(Some(Value::Angka(s.chars().count() as i64))),
            Some(Value::Peta(p)) => Ok(Some(Value::Angka(p.len() as i64))),
            Some(lain) => Err(format!("panjang() tidak berlaku untuk {}", lain)),
            None => Err("panjang() butuh 1 argumen".to_string()),
        },
        "gabung" => {
            let daftar = args.get(0).ok_or_else(|| "gabung(daftar, item) butuh 2 argumen".to_string())?;
            let item = args.get(1).ok_or_else(|| "gabung(daftar, item) butuh 2 argumen".to_string())?;
            match daftar_materialisasi(daftar) {
                Some(d) => { let mut baru = (*d).clone(); baru.push(item.clone()); Ok(Some(buat_daftar(baru))) }
                None => Err(format!("gabung() argumen pertama harus Daftar, ditemukan {}", daftar)),
            }
        }
        "ambil" => {
            let struktur = args.get(0).ok_or_else(|| "ambil(struktur, kunci) butuh 2 argumen".to_string())?;
            let kunci = args.get(1).ok_or_else(|| "ambil(struktur, kunci) butuh 2 argumen".to_string())?;
            match (struktur, kunci) {
                (Value::Daftar(d), Value::Angka(i)) => {
                    if *i < 0 { return Err(format!("Indeks tidak boleh negatif: {}", i)); }
                    d.get(*i as usize).cloned().map(Some).ok_or_else(|| format!("Indeks {} di luar jangkauan (panjang daftar: {})", i, d.len()))
                }
                (Value::DaftarAngka(d), Value::Angka(i)) => {
                    if *i < 0 { return Err(format!("Indeks tidak boleh negatif: {}", i)); }
                    d.get(*i as usize).map(|n| Value::Angka(*n)).map(Some).ok_or_else(|| format!("Indeks {} di luar jangkauan (panjang daftar: {})", i, d.len()))
                }
                (Value::DaftarDesimal(d), Value::Angka(i)) => {
                    if *i < 0 { return Err(format!("Indeks tidak boleh negatif: {}", i)); }
                    d.get(*i as usize).map(|x| Value::Desimal(*x)).map(Some).ok_or_else(|| format!("Indeks {} di luar jangkauan (panjang daftar: {})", i, d.len()))
                }
                (Value::Peta(entries), Value::Teks(k)) => {
                    entries.iter().find(|(kk, _)| kk.as_ref() == k.as_ref()).map(|(_, v)| v.clone()).map(Some)
                        .ok_or_else(|| format!("Kunci \"{}\" tidak ditemukan di Peta.", k))
                }
                (s, k) => Err(format!("ambil() butuh (Daftar, Angka) atau (Peta, Teks), ditemukan {} dan {}", s, k)),
            }
        }
        "jumlah" => match args.get(0) {
            // Jalur cepat: representasi flat, langsung sum tanpa cek tag per elemen --
            // ini yang di-autovectorize compiler jadi SIMD (lihat komentar di enum Value).
            Some(Value::DaftarAngka(d)) => Ok(Some(Value::Angka(d.iter().sum()))),
            Some(Value::DaftarDesimal(d)) => Ok(Some(Value::Desimal(d.iter().sum()))),
            Some(Value::Daftar(d)) => {
                let mut total_i = 0i64; let mut total_f = 0f64; let mut ada_desimal = false;
                for v in d.iter() {
                    match v {
                        Value::Angka(n) => { total_i += n; total_f += *n as f64; }
                        Value::Desimal(x) => { ada_desimal = true; total_f += x; }
                        lain => return Err(format!("jumlah() hanya untuk daftar berisi Angka, ditemukan {}", lain)),
                    }
                }
                if ada_desimal { Ok(Some(Value::Desimal(total_f))) } else { Ok(Some(Value::Angka(total_i))) }
            }
            Some(lain) => Err(format!("jumlah() butuh Daftar, ditemukan {}", lain)),
            None => Err("jumlah() butuh 1 argumen".to_string()),
        },
        "rata_rata" => match args.get(0) {
            Some(Value::DaftarAngka(d)) if !d.is_empty() => {
                let total: i64 = d.iter().sum();
                Ok(Some(Value::Angka(total / d.len() as i64)))
            }
            Some(Value::DaftarDesimal(d)) if !d.is_empty() => {
                let total: f64 = d.iter().sum();
                Ok(Some(Value::Desimal(total / d.len() as f64)))
            }
            Some(Value::DaftarAngka(_)) | Some(Value::DaftarDesimal(_)) => Err("rata_rata() tidak bisa dihitung dari daftar kosong".to_string()),
            Some(Value::Daftar(d)) if !d.is_empty() => {
                let mut total_i = 0i64; let mut total_f = 0f64; let mut ada_desimal = false;
                for v in d.iter() {
                    match v {
                        Value::Angka(n) => { total_i += n; total_f += *n as f64; }
                        Value::Desimal(x) => { ada_desimal = true; total_f += x; }
                        lain => return Err(format!("rata_rata() hanya untuk daftar berisi Angka, ditemukan {}", lain)),
                    }
                }
                if ada_desimal { Ok(Some(Value::Desimal(total_f / d.len() as f64))) } else { Ok(Some(Value::Angka(total_i / d.len() as i64))) }
            }
            Some(Value::Daftar(_)) => Err("rata_rata() tidak bisa dihitung dari daftar kosong".to_string()),
            Some(lain) => Err(format!("rata_rata() butuh Daftar, ditemukan {}", lain)),
            None => Err("rata_rata() butuh 1 argumen".to_string()),
        },
        "kunci_peta" => match args.get(0) {
            Some(Value::Peta(entries)) => Ok(Some(Value::Daftar(Rc::new(entries.iter().map(|(k, _)| Value::Teks(k.clone().into())).collect())))),
            Some(lain) => Err(format!("kunci_peta() butuh Peta, ditemukan {}", lain)),
            None => Err("kunci_peta() butuh 1 argumen".to_string()),
        },
        "urai_json" => match args.get(0) {
            Some(Value::Teks(s)) => json_urai(s).map(Some),
            Some(lain) => Err(format!("urai_json() butuh Teks, ditemukan {}", lain)),
            None => Err("urai_json(teks) butuh 1 argumen".to_string()),
        },
        "teks_json" => match args.get(0) {
            Some(v) => Ok(Some(Value::Teks(json_dari_value(v).into()))),
            None => Err("teks_json(nilai) butuh 1 argumen".to_string()),
        },
        // Milestone C: dipakai `isoteri uji` (test runner minimal, lihat main.rs mode_uji) --
        // konvensi: `kalau (bukan kondisi) { gagal_uji("pesan") }`. Cuma melempar Err biasa,
        // sama seperti error runtime lain -- gak butuh mekanisme baru sama sekali.
        "gagal_uji" => match args.get(0) {
            Some(Value::Teks(pesan)) => Err(pesan.to_string()),
            Some(lain) => Err(format!("gagal_uji(): {}", lain)),
            None => Err("Uji gagal (gagal_uji() dipanggil tanpa pesan).".to_string()),
        },
        "baca_berkas" => match args.get(0) {
            Some(Value::Teks(p)) => match fs::read_to_string(p.as_ref()) {
                Ok(isi) => Ok(Some(Value::Teks(isi.into()))),
                Err(e) => Err(format!("Tidak bisa membaca berkas \"{}\": {}", p, e)),
            },
            Some(lain) => Err(format!("baca_berkas() butuh nama berkas berupa Teks, ditemukan {}", lain)),
            None => Err("baca_berkas(path) butuh 1 argumen".to_string()),
        },
        "tulis_berkas" => {
            let path = args.get(0).ok_or_else(|| "tulis_berkas(path, isi) butuh 2 argumen".to_string())?;
            let isi = args.get(1).ok_or_else(|| "tulis_berkas(path, isi) butuh 2 argumen".to_string())?;
            match (path, isi) {
                (Value::Teks(p), Value::Teks(s)) => match fs::write(p.as_ref(), s.as_ref()) {
                    Ok(_) => Ok(Some(Value::Bool(true))),
                    Err(e) => Err(format!("Tidak bisa menulis berkas \"{}\": {}", p, e)),
                },
                (p, s) => Err(format!("tulis_berkas(path, isi) butuh dua Teks, ditemukan {} dan {}", p, s)),
            }
        }
        #[cfg(feature = "native-http")]
        "unduh" => match args.get(0) {
            Some(Value::Teks(u)) => match ureq::get(u.as_ref()).call() {
                Ok(resp) => match resp.into_string() {
                    Ok(body) => Ok(Some(Value::Teks(body.into()))),
                    Err(e) => Err(format!("Gagal membaca respons dari \"{}\": {}", u, e)),
                },
                Err(e) => Err(format!("Gagal mengunduh dari \"{}\": {}", u, e)),
            },
            Some(lain) => Err(format!("unduh() butuh URL berupa Teks, ditemukan {}", lain)),
            None => Err("unduh(url) butuh 1 argumen".to_string()),
        },
        // Build tanpa fitur "native-http" (mis. isoteri-wasm/, jalur ekspor JSON untuk web) --
        // browser sudah punya fetch()/unduh_async() sendiri (lihat runtime/web/isoteri-vm.js),
        // jadi unduh() blocking gaya native memang TIDAK RELEVAN & TIDAK BISA jalan di sana
        // (butuh socket blocking asli yang gak ada di wasm32/browser) -- error jelas, bukan
        // silent-gagal-kompilasi programnya.
        #[cfg(not(feature = "native-http"))]
        "unduh" => Err("unduh() (versi blocking) tidak tersedia di build ini (mis. isoteri-wasm/) -- pakai unduh_async()/unduh_lanjut_async() kalau target-nya web.".to_string()),
        // Dipakai bareng server_mulai() -- handler yang mau balikin status code SELAIN 200
        // bungkus nilainya lewat ini. Tanpa ini, handler yang cuma balikin Teks/Peta biasa
        // otomatis dianggap status 200 (lihat respons_dari_value() di dekat jalankan_http_server()).
        "respons_status" => {
            let kode = match args.get(0) {
                Some(Value::Angka(n)) => *n,
                Some(lain) => return Err(format!("respons_status(kode, nilai): argumen pertama (kode status) harus Angka, ditemukan {}", lain)),
                None => return Err("respons_status(kode, nilai) butuh 2 argumen".to_string()),
            };
            if !(100..=599).contains(&kode) {
                return Err(format!("respons_status(): kode status HTTP harus 100-599, ditemukan {}", kode));
            }
            let nilai = args.get(1).cloned().ok_or_else(|| "respons_status(kode, nilai) butuh 2 argumen".to_string())?;
            Ok(Some(Value::Instans(Rc::from("ResponsHttp"), Rc::new(vec![
                ("status".into(), Value::Angka(kode)),
                ("nilai".into(), nilai),
            ]))))
        }
        "ke_desimal" => match args.get(0) {
            Some(Value::Angka(n)) => Ok(Some(Value::Desimal(*n as f64))),
            Some(v @ Value::Desimal(_)) => Ok(Some(v.clone())),
            Some(lain) => Err(format!("ke_desimal() tidak berlaku untuk {}", lain)),
            None => Err("ke_desimal(angka) butuh 1 argumen".to_string()),
        },
        "ke_bulat" => match args.get(0) {
            Some(Value::Desimal(f)) => Ok(Some(Value::Angka(*f as i64))),
            Some(v @ Value::Angka(_)) => Ok(Some(v.clone())),
            Some(lain) => Err(format!("ke_bulat() tidak berlaku untuk {}", lain)),
            None => Err("ke_bulat(desimal) butuh 1 argumen".to_string()),
        },
        "ke_angka" => match args.get(0) {
            Some(v @ Value::Angka(_)) => Ok(Some(v.clone())),
            Some(Value::Desimal(f)) => Ok(Some(Value::Angka(*f as i64))),
            Some(Value::Teks(s)) => match s.trim().parse::<i64>() {
                Ok(n) => Ok(Some(Value::Angka(n))),
                Err(_) => Err(format!("ke_angka(): \"{}\" bukan Angka (i64) yang valid.", s)),
            },
            Some(lain) => Err(format!("ke_angka() tidak berlaku untuk {}", lain)),
            None => Err("ke_angka(nilai) butuh 1 argumen".to_string()),
        },
        "ke_teks" => match args.get(0) {
            Some(v) => Ok(Some(Value::Teks(v.to_string().into()))),
            None => Err("ke_teks(nilai) butuh 1 argumen".to_string()),
        },

        // --- Matematika ---
        "akar" => match ke_desimal(args.get(0).ok_or_else(|| "akar(angka) butuh 1 argumen".to_string())?) {
            Some(x) if x >= 0.0 => Ok(Some(Value::Desimal(x.sqrt()))),
            Some(_) => Err("akar() tidak berlaku untuk angka negatif.".to_string()),
            None => Err(format!("akar() butuh Angka/Desimal, ditemukan {}", args[0])),
        },
        "pangkat" => {
            let basis = args.get(0).ok_or_else(|| "pangkat(basis, eksponen) butuh 2 argumen".to_string())?;
            let eksponen = args.get(1).ok_or_else(|| "pangkat(basis, eksponen) butuh 2 argumen".to_string())?;
            match (basis, eksponen) {
                (Value::Angka(b), Value::Angka(e)) if *e >= 0 => Ok(Some(Value::Angka(b.pow(*e as u32)))),
                _ => match (ke_desimal(basis), ke_desimal(eksponen)) {
                    (Some(b), Some(e)) => Ok(Some(Value::Desimal(b.powf(e)))),
                    _ => Err(format!("pangkat() butuh Angka/Desimal, ditemukan {} dan {}", basis, eksponen)),
                },
            }
        }
        "bulat" => match ke_desimal(args.get(0).ok_or_else(|| "bulat(desimal) butuh 1 argumen".to_string())?) {
            Some(x) => Ok(Some(Value::Angka(x.round() as i64))),
            None => Err(format!("bulat() butuh Angka/Desimal, ditemukan {}", args[0])),
        },
        "bulat_bawah" => match ke_desimal(args.get(0).ok_or_else(|| "bulat_bawah(desimal) butuh 1 argumen".to_string())?) {
            Some(x) => Ok(Some(Value::Angka(x.floor() as i64))),
            None => Err(format!("bulat_bawah() butuh Angka/Desimal, ditemukan {}", args[0])),
        },
        "bulat_atas" => match ke_desimal(args.get(0).ok_or_else(|| "bulat_atas(desimal) butuh 1 argumen".to_string())?) {
            Some(x) => Ok(Some(Value::Angka(x.ceil() as i64))),
            None => Err(format!("bulat_atas() butuh Angka/Desimal, ditemukan {}", args[0])),
        },
        "mutlak" => match args.get(0) {
            Some(Value::Angka(n)) => Ok(Some(Value::Angka(n.abs()))),
            Some(Value::Desimal(f)) => Ok(Some(Value::Desimal(f.abs()))),
            Some(lain) => Err(format!("mutlak() tidak berlaku untuk {}", lain)),
            None => Err("mutlak(angka) butuh 1 argumen".to_string()),
        },
        "min" => {
            let a = args.get(0).ok_or_else(|| "min(a, b) butuh 2 argumen".to_string())?;
            let b = args.get(1).ok_or_else(|| "min(a, b) butuh 2 argumen".to_string())?;
            match (ke_desimal(a), ke_desimal(b)) {
                (Some(x), Some(y)) => Ok(Some(if x <= y { a.clone() } else { b.clone() })),
                _ => Err(format!("min() butuh Angka/Desimal, ditemukan {} dan {}", a, b)),
            }
        }
        "maks" => {
            let a = args.get(0).ok_or_else(|| "maks(a, b) butuh 2 argumen".to_string())?;
            let b = args.get(1).ok_or_else(|| "maks(a, b) butuh 2 argumen".to_string())?;
            match (ke_desimal(a), ke_desimal(b)) {
                (Some(x), Some(y)) => Ok(Some(if x >= y { a.clone() } else { b.clone() })),
                _ => Err(format!("maks() butuh Angka/Desimal, ditemukan {} dan {}", a, b)),
            }
        }
        "acak" => Ok(Some(Value::Desimal(acak_f64()))),

        // --- Teks ---
        "potong" => {
            let s = match args.get(0) { Some(Value::Teks(s)) => s, _ => return Err("potong(teks, mulai, akhir) argumen pertama harus Teks".to_string()) };
            let mulai = match args.get(1) { Some(Value::Angka(n)) => *n, _ => return Err("potong(teks, mulai, akhir) butuh Angka untuk 'mulai'".to_string()) };
            let akhir = match args.get(2) { Some(Value::Angka(n)) => *n, _ => return Err("potong(teks, mulai, akhir) butuh Angka untuk 'akhir'".to_string()) };
            let chars: Vec<char> = s.chars().collect();
            let mulai = mulai.max(0) as usize;
            let akhir = (akhir.max(0) as usize).min(chars.len());
            if mulai > akhir { return Err(format!("potong(): 'mulai' ({}) tidak boleh lebih besar dari 'akhir' ({})", mulai, akhir)); }
            Ok(Some(Value::Teks(chars[mulai..akhir].iter().collect::<String>().into())))
        }
        "ganti" => {
            let s = match args.get(0) { Some(Value::Teks(s)) => s, _ => return Err("ganti(teks, dari, ke) argumen pertama harus Teks".to_string()) };
            let dari = match args.get(1) { Some(Value::Teks(s)) => s, _ => return Err("ganti(teks, dari, ke) butuh Teks untuk 'dari'".to_string()) };
            let ke = match args.get(2) { Some(Value::Teks(s)) => s, _ => return Err("ganti(teks, dari, ke) butuh Teks untuk 'ke'".to_string()) };
            Ok(Some(Value::Teks(s.replace(dari.as_ref(), ke).into())))
        }
        "huruf_besar" => match args.get(0) {
            Some(Value::Teks(s)) => Ok(Some(Value::Teks(s.to_uppercase().into()))),
            Some(lain) => Err(format!("huruf_besar() butuh Teks, ditemukan {}", lain)),
            None => Err("huruf_besar(teks) butuh 1 argumen".to_string()),
        },
        "huruf_kecil" => match args.get(0) {
            Some(Value::Teks(s)) => Ok(Some(Value::Teks(s.to_lowercase().into()))),
            Some(lain) => Err(format!("huruf_kecil() butuh Teks, ditemukan {}", lain)),
            None => Err("huruf_kecil(teks) butuh 1 argumen".to_string()),
        },
        "pangkas" => match args.get(0) {
            Some(Value::Teks(s)) => Ok(Some(Value::Teks(s.trim().into()))),
            Some(lain) => Err(format!("pangkas() butuh Teks, ditemukan {}", lain)),
            None => Err("pangkas(teks) butuh 1 argumen".to_string()),
        },
        "pisah" => {
            let s = match args.get(0) { Some(Value::Teks(s)) => s, _ => return Err("pisah(teks, pemisah) argumen pertama harus Teks".to_string()) };
            let pemisah = match args.get(1) { Some(Value::Teks(s)) => s, _ => return Err("pisah(teks, pemisah) butuh Teks untuk 'pemisah'".to_string()) };
            let bagian: Vec<Value> = if pemisah.is_empty() {
                s.chars().map(|c| Value::Teks(c.to_string().into())).collect()
            } else {
                s.split(pemisah.as_ref()).map(|b| Value::Teks(b.into())).collect()
            };
            Ok(Some(Value::Daftar(Rc::new(bagian))))
        }
        "satukan" => {
            let daftar = match args.get(0).and_then(daftar_materialisasi) { Some(d) => d, None => return Err("satukan(daftar, pemisah) argumen pertama harus Daftar".to_string()) };
            let pemisah = match args.get(1) { Some(Value::Teks(s)) => s, _ => return Err("satukan(daftar, pemisah) butuh Teks untuk 'pemisah'".to_string()) };
            let mut potongan_teks = Vec::with_capacity(daftar.len());
            for v in daftar.iter() {
                match v {
                    Value::Teks(s) => potongan_teks.push(s.to_string()),
                    lain => return Err(format!("satukan() cuma bisa buat daftar berisi Teks, ditemukan {}", lain)),
                }
            }
            Ok(Some(Value::Teks(potongan_teks.join(pemisah).into())))
        }
        "mengandung" => {
            let s = match args.get(0) { Some(Value::Teks(s)) => s, _ => return Err("mengandung(teks, sub) argumen pertama harus Teks".to_string()) };
            let sub = match args.get(1) { Some(Value::Teks(s)) => s, _ => return Err("mengandung(teks, sub) butuh Teks untuk 'sub'".to_string()) };
            Ok(Some(Value::Bool(s.contains(sub.as_ref()))))
        }
        "diawali" => {
            let s = match args.get(0) { Some(Value::Teks(s)) => s, _ => return Err("diawali(teks, awalan) argumen pertama harus Teks".to_string()) };
            let awalan = match args.get(1) { Some(Value::Teks(s)) => s, _ => return Err("diawali(teks, awalan) butuh Teks untuk 'awalan'".to_string()) };
            Ok(Some(Value::Bool(s.starts_with(awalan.as_ref()))))
        }
        "diakhiri" => {
            let s = match args.get(0) { Some(Value::Teks(s)) => s, _ => return Err("diakhiri(teks, akhiran) argumen pertama harus Teks".to_string()) };
            let akhiran = match args.get(1) { Some(Value::Teks(s)) => s, _ => return Err("diakhiri(teks, akhiran) butuh Teks untuk 'akhiran'".to_string()) };
            Ok(Some(Value::Bool(s.ends_with(akhiran.as_ref()))))
        }

        _ => Ok(None),
    }
}

/// Generator angka acak sederhana (xorshift64), zero-dependency -- cukup buat kebutuhan skrip
/// biasa (bukan kriptografi). State-nya statis & thread-safe (AtomicU64), dibarui tiap panggilan.
fn acak_f64() -> f64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static STATE: AtomicU64 = AtomicU64::new(0);
    let mut lama = STATE.load(Ordering::Relaxed);
    if lama == 0 {
        lama = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0x2545F4914F6CDD1D) | 1;
    }
    let mut x = lama;
    x ^= x << 13; x ^= x >> 7; x ^= x << 17;
    STATE.store(x, Ordering::Relaxed);
    // Ambil 53 bit atas jadi mantissa f64, hasilkan pecahan di [0, 1).
    (x >> 11) as f64 / (1u64 << 53) as f64
}

// =====================================================================
// 7. JSON: parser & serializer, zero-dependency
// =====================================================================

fn json_urai(s: &str) -> Result<Value, String> {
    let chars: Vec<char> = s.chars().collect();
    let mut pos = 0usize;
    json_skip_ws(&chars, &mut pos);
    json_nilai(&chars, &mut pos)
}
fn json_skip_ws(c: &[char], pos: &mut usize) { while *pos < c.len() && c[*pos].is_whitespace() { *pos += 1; } }
fn json_nilai(c: &[char], pos: &mut usize) -> Result<Value, String> {
    json_skip_ws(c, pos);
    if *pos >= c.len() { return Err("JSON tidak lengkap.".to_string()); }
    match c[*pos] {
        '{' => json_objek(c, pos),
        '[' => json_larik(c, pos),
        '"' => Ok(Value::Teks(json_string(c, pos)?.into())),
        't' => { json_harap_kata(c, pos, "true")?; Ok(Value::Bool(true)) }
        'f' => { json_harap_kata(c, pos, "false")?; Ok(Value::Bool(false)) }
        'n' => { json_harap_kata(c, pos, "null")?; Ok(Value::Kosong) }
        _ => json_angka(c, pos),
    }
}
fn json_harap_kata(c: &[char], pos: &mut usize, kata: &str) -> Result<(), String> {
    for ch in kata.chars() {
        if *pos >= c.len() || c[*pos] != ch { return Err(format!("JSON tidak valid, diharapkan \"{}\".", kata)); }
        *pos += 1;
    }
    Ok(())
}
fn json_string(c: &[char], pos: &mut usize) -> Result<String, String> {
    *pos += 1;
    let mut s = String::new();
    while *pos < c.len() && c[*pos] != '"' {
        if c[*pos] == '\\' && *pos + 1 < c.len() {
            match c[*pos + 1] {
                '"' => { s.push('"'); *pos += 2; }
                '\\' => { s.push('\\'); *pos += 2; }
                'n' => { s.push('\n'); *pos += 2; }
                't' => { s.push('\t'); *pos += 2; }
                'r' => { s.push('\r'); *pos += 2; }
                '/' => { s.push('/'); *pos += 2; }
                'u' => {
                    if *pos + 5 < c.len() {
                        let hex: String = c[*pos + 2..*pos + 6].iter().collect();
                        if let Ok(code) = u32::from_str_radix(&hex, 16) { if let Some(ch) = char::from_u32(code) { s.push(ch); } }
                        *pos += 6;
                    } else { *pos += 2; }
                }
                lain => { s.push(lain); *pos += 2; }
            }
        } else { s.push(c[*pos]); *pos += 1; }
    }
    if *pos >= c.len() { return Err("Teks JSON tidak ditutup dengan tanda kutip.".to_string()); }
    *pos += 1;
    Ok(s)
}
fn json_angka(c: &[char], pos: &mut usize) -> Result<Value, String> {
    let mulai = *pos;
    if *pos < c.len() && c[*pos] == '-' { *pos += 1; }
    while *pos < c.len() && c[*pos].is_ascii_digit() { *pos += 1; }
    let mut desimal = false;
    if *pos < c.len() && c[*pos] == '.' { desimal = true; *pos += 1; while *pos < c.len() && c[*pos].is_ascii_digit() { *pos += 1; } }
    if *pos < c.len() && (c[*pos] == 'e' || c[*pos] == 'E') {
        desimal = true; *pos += 1;
        if *pos < c.len() && (c[*pos] == '+' || c[*pos] == '-') { *pos += 1; }
        while *pos < c.len() && c[*pos].is_ascii_digit() { *pos += 1; }
    }
    let teks: String = c[mulai..*pos].iter().collect();
    if teks.is_empty() || teks == "-" { return Err("JSON tidak valid: angka kosong.".to_string()); }
    if desimal { teks.parse::<f64>().map(Value::Desimal).map_err(|_| "JSON tidak valid: format angka desimal salah.".to_string()) }
    else { teks.parse::<i64>().map(Value::Angka).map_err(|_| "JSON tidak valid: format angka bulat salah.".to_string()) }
}
fn json_larik(c: &[char], pos: &mut usize) -> Result<Value, String> {
    *pos += 1;
    let mut items = Vec::new();
    json_skip_ws(c, pos);
    if *pos < c.len() && c[*pos] == ']' { *pos += 1; return Ok(Value::Daftar(items.into())); }
    loop {
        items.push(json_nilai(c, pos)?);
        json_skip_ws(c, pos);
        if *pos < c.len() && c[*pos] == ',' { *pos += 1; json_skip_ws(c, pos); continue; }
        break;
    }
    json_skip_ws(c, pos);
    if *pos >= c.len() || c[*pos] != ']' { return Err("JSON larik tidak ditutup dengan ']'.".to_string()); }
    *pos += 1;
    Ok(Value::Daftar(items.into()))
}
fn json_objek(c: &[char], pos: &mut usize) -> Result<Value, String> {
    *pos += 1;
    let mut entries = Vec::new();
    json_skip_ws(c, pos);
    if *pos < c.len() && c[*pos] == '}' { *pos += 1; return Ok(Value::Peta(entries.into())); }
    loop {
        json_skip_ws(c, pos);
        if *pos >= c.len() || c[*pos] != '"' { return Err("JSON objek: kunci harus berupa teks berpetik dua.".to_string()); }
        let kunci = json_string(c, pos)?;
        json_skip_ws(c, pos);
        if *pos >= c.len() || c[*pos] != ':' { return Err("JSON objek: diharapkan ':' setelah kunci.".to_string()); }
        *pos += 1;
        let nilai = json_nilai(c, pos)?;
        entries.push((kunci.into(), nilai));
        json_skip_ws(c, pos);
        if *pos < c.len() && c[*pos] == ',' { *pos += 1; continue; }
        break;
    }
    json_skip_ws(c, pos);
    if *pos >= c.len() || c[*pos] != '}' { return Err("JSON objek tidak ditutup dengan '}'.".to_string()); }
    *pos += 1;
    Ok(Value::Peta(entries.into()))
}
fn json_escape(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""), '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"), '\t' => out.push_str("\\t"), '\r' => out.push_str("\\r"),
            _ => out.push(ch),
        }
    }
    out
}
fn json_dari_value(v: &Value) -> String {
    match v {
        Value::Angka(n) => n.to_string(),
        Value::Desimal(f) => f.to_string(),
        Value::Teks(s) => format!("\"{}\"", json_escape(s)),
        Value::Bool(b) => if *b { "true".to_string() } else { "false".to_string() },
        Value::Daftar(items) => format!("[{}]", items.iter().map(json_dari_value).collect::<Vec<_>>().join(",")),
        Value::DaftarAngka(items) => format!("[{}]", items.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",")),
        Value::DaftarDesimal(items) => format!("[{}]", items.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")),
        Value::Peta(entries) => format!("{{{}}}", entries.iter().map(|(k, v)| format!("\"{}\":{}", json_escape(k), json_dari_value(v))).collect::<Vec<_>>().join(",")),
        Value::Kosong => "null".to_string(),
        Value::Instans(_, entries) => format!("{{{}}}", entries.iter().map(|(k, v)| format!("\"{}\":{}", json_escape(k), json_dari_value(v))).collect::<Vec<_>>().join(",")),
        Value::Fungsi(_) => "null".to_string(), // fungsi gak bisa direpresentasikan di JSON
    }
}

/// server_mulai(port, handler) -- HTTP server BLOCKING (sengaja, konsisten dengan model
/// eksekusi VM yang sinkron -- sama seperti unduh(); TIDAK ada runtime async/tokio). Prioritas
/// #2 di "Arah strategis" ROADMAP.md: prasyarat mutlak buat pola "satu skema, dua sisi" --
/// bentuk + fungsi validasi yang sama bisa di-'muat' dari backend (di sini) DAN frontend
/// (browser, lewat ekspor-web) tanpa duplikasi/drift.
///
/// `handler` dipanggil (lewat panggil_callback_1_arg, mekanisme SAMA dipakai petakan/saring/
/// urutkan) satu kali per request masuk, dengan SATU argumen: Peta berisi field "metode"
/// (Teks), "path" (Teks, tanpa query string), "query" (Peta), "header" (Peta), "body" (Teks).
/// Nilai balik handler diinterpretasi lewat respons_dari_value() -- lihat di situ.
#[cfg(feature = "native-server")]
fn jalankan_http_server(pustaka: &Pustaka, state: &mut VMState, port: u16, handler: &Value) -> Result<(), String> {
    let alamat = format!("0.0.0.0:{}", port);
    let server = tiny_http::Server::http(&alamat)
        .map_err(|e| format!("server_mulai(): gagal membuka port {}: {}", port, e))?;
    eprintln!("Server Isoteri jalan di http://localhost:{}/ (Ctrl+C buat berhenti)", port);

    for mut permintaan in server.incoming_requests() {
        let metode = permintaan.method().to_string();
        let url = permintaan.url().to_string();
        let (path, query_str) = match url.split_once('?') {
            Some((p, q)) => (p.to_string(), Some(q.to_string())),
            None => (url.clone(), None),
        };
        let query_peta: Vec<(Rc<str>, Value)> = query_str.map(|q| {
            q.split('&').filter_map(|pasangan| {
                if pasangan.is_empty() { return None; }
                let mut it = pasangan.splitn(2, '=');
                let k: Rc<str> = it.next()?.into();
                let v = it.next().unwrap_or("").to_string();
                Some((k, Value::Teks(v.into())))
            }).collect()
        }).unwrap_or_default();
        let header_peta: Vec<(Rc<str>, Value)> = permintaan.headers().iter()
            .map(|h| (Rc::from(h.field.as_str().as_str()), Value::Teks(h.value.as_str().to_string().into())))
            .collect();

        let mut body = String::new();
        use std::io::Read;
        let _ = permintaan.as_reader().read_to_string(&mut body);

        let req_peta = Value::Peta(Rc::new(vec![
            ("metode".into(), Value::Teks(metode.into())),
            ("path".into(), Value::Teks(path.into())),
            ("query".into(), Value::Peta(Rc::new(query_peta))),
            ("header".into(), Value::Peta(Rc::new(header_peta))),
            ("body".into(), Value::Teks(body.into())),
        ]));

        let hasil = match panggil_callback_1_arg(pustaka, state, "server_mulai", handler, req_peta) {
            Ok(v) => v,
            Err(e) => {
                // Error di handler TIDAK menghentikan server -- satu request gagal jangan
                // sampai membunuh proses server buat semua request lain. Log ke stderr,
                // balas 500 ke klien yang bersangkutan.
                eprintln!("Kesalahan di dalam handler server_mulai(): {}", e);
                let _ = permintaan.respond(tiny_http::Response::from_string(
                    format!("{{\"error\":\"{}\"}}", json_escape(&e))
                ).with_status_code(500)
                .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
                .with_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap()));
                continue;
            }
        };

        let (status_kode, body_teks, content_type) = respons_dari_value(&hasil);
        let respons = tiny_http::Response::from_string(body_teks)
            .with_status_code(status_kode)
            .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()).unwrap())
            .with_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap())
            .with_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, PUT, DELETE, OPTIONS"[..]).unwrap())
            .with_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Headers"[..], &b"Content-Type"[..]).unwrap());
        let _ = permintaan.respond(respons);
    }
    Ok(())
}

#[cfg(not(feature = "native-server"))]
fn jalankan_http_server(_pustaka: &Pustaka, _state: &mut VMState, _port: u16, _handler: &Value) -> Result<(), String> {
    Err("server_mulai() tidak tersedia di build ini (mis. isoteri-wasm/) -- HTTP server native butuh socket asli yang gak ada di browser.".to_string())
}

/// Terjemahkan nilai balik handler server_mulai() jadi (kode status, body teks, content-type).
/// - Instans "ResponsHttp" (dibuat lewat respons_status()) -> pakai kode statusnya, lalu
///   proses field "nilai" secara REKURSIF lewat aturan yang sama di bawah.
/// - Teks -> 200, text/plain (body = teks itu apa adanya).
/// - Kosong -> 204 No Content, body kosong.
/// - Selain itu (Peta/Daftar/Instans/Angka/dst) -> 200, application/json, di-serialize lewat
///   json_dari_value() -- REUSE mesin JSON yang sama dipakai tulis_berkas(), bukan encoder baru.
#[cfg(feature = "native-server")]
fn respons_dari_value(v: &Value) -> (u16, String, &'static str) {
    match v {
        Value::Instans(nama, entries) if nama.as_ref() == "ResponsHttp" => {
            let kode = entries.iter().find(|(k, _)| k.as_ref() == "status")
                .and_then(|(_, v)| if let Value::Angka(n) = v { Some(*n as u16) } else { None })
                .unwrap_or(200);
            let nilai = entries.iter().find(|(k, _)| k.as_ref() == "nilai").map(|(_, v)| v.clone()).unwrap_or(Value::Kosong);
            let (_, body, ct) = respons_dari_value(&nilai);
            (kode, body, ct)
        }
        Value::Teks(s) => (200, s.to_string(), "text/plain; charset=utf-8"),
        Value::Kosong => (204, String::new(), "text/plain"),
        lain => (200, json_dari_value(lain), "application/json"),
    }
}

// =====================================================================
// 8. MAIN
// =====================================================================

/// Ekspansi statement 'muat "path.iso"' di level atas program -- jalan SEBELUM resolver,
/// jadi Resolver/Compiler/VM sama sekali tidak tahu soal modul, mereka cuma lihat satu
/// Vec<Stmt> gabungan seolah-olah semua ditulis di satu file. Path relatif dihitung dari
/// direktori file YANG MEMUAT (bukan selalu dari file utama), jadi modul juga bisa
// =====================================================================
// 10. PACKAGE MANAGER MINIMAL (Milestone C, docs/FILOSOFI.md)
// =====================================================================
//
// Sengaja MINIMAL sesuai filosofi ("bukan langsung sekelas Cargo/npm"):
// dependensi lokal lewat path (belum ada registry -- itu memang item
// TERPISAH & belakangan di roadmap Milestone C, bukan bagian dari v1 ini).
// Manifest (`isoteri.toml`) di-parse dengan parser tulisan tangan (bukan
// dependensi crate `toml`) -- skemanya sengaja cuma dua level (key=value
// datar + SATU section `[dependensi]`), konsisten dengan gaya proyek ini
// yang juga menulis parser JSON sendiri (lihat json_urai) daripada
// menambah dependensi buat sesuatu yang skema-nya dikontrol sendiri.
//
// Konvensi paket: satu direktori berisi `isoteri.toml` (nama, versi) DAN
// `src/main.iso` (entry point buat dijalankan langsung) ATAU `src/lib.iso`
// (buat dijadikan dependensi paket lain, dimuat lewat namanya, bukan path
// relatif). `isoteri tambah nama path/ke/paket` mendaftarkan pemetaan nama
// -> path di `[dependensi]`; `muat "nama"` (tanpa `/` atau akhiran `.iso`)
// dicoba diresolusi lewat manifest SEBELUM dicoba sebagai path relatif
// biasa gagal -- lihat `resolusi_muat`.

/// Sumber sebuah dependensi di `[dependensi]` -- dua cara mendapatkan paket:
/// - `Lokal`: path direktori paket relatif terhadap lokasi isoteri.toml (perilaku lama,
///   TIDAK berubah). Cocok buat paket yang dikembangkan bareng dalam satu monorepo/mesin.
/// - `Git`: registry v1 (lihat docs/FILOSOFI.md Milestone C) -- paket = repo Git APAPUN
///   (GitHub/GitLab/Gitea/dst, bukan indeks server terpusat kayak npm/Cargo) yang dipin ke
///   satu `tag` (rilis semver, direkomendasikan) ATAU `rev` (commit hash spesifik), TIDAK
///   PERNAH KEDUANYA. Diresolusi lewat `resolusi_paket_git` ke cache lokal.
#[derive(Debug, Clone, PartialEq)]
pub enum SumberDependensi {
    Lokal(String),
    Git { url: String, tag: Option<String>, rev: Option<String> },
}

#[derive(Debug, Clone, Default)]
pub struct Manifest {
    pub nama: String,
    pub versi: String,
    /// nama_paket -> sumbernya (path lokal ATAU repo git+tag/rev)
    pub dependensi: HashMap<String, SumberDependensi>,
}

/// Uraikan isi `{ kunci = "nilai", kunci2 = "nilai2" }` jadi pasangan (kunci, nilai).
/// Pemisah `,` di level atas saja -- cukup buat skema flat yang dipakai isoteri.toml,
/// tidak perlu penanganan koma di dalam string (URL git tidak pernah mengandung koma).
fn urai_pasangan_kurung(dalam: &str, no: usize) -> Result<Vec<(String, String)>, String> {
    let mut hasil = Vec::new();
    for bagian in dalam.split(',') {
        let bagian = bagian.trim();
        if bagian.is_empty() { continue; }
        let (k, v) = bagian.split_once('=')
            .ok_or_else(|| format!("isoteri.toml baris {}: pasangan \"{}\" tidak valid di dalam {{ }}.", no, bagian))?;
        let v = urai_string_toml(v.trim())
            .ok_or_else(|| format!("isoteri.toml baris {}: nilai \"{}\" harus Teks diapit tanda kutip dua.", no, v.trim()))?;
        hasil.push((k.trim().to_string(), v));
    }
    Ok(hasil)
}

/// Parser tulisan tangan buat subset TOML yang dipakai `isoteri.toml`:
/// - baris kosong & komentar (`#...`) diabaikan
/// - `kunci = "nilai"` di level atas (sebelum section manapun) -> field manifest
/// - `[dependensi]` menandai section; baris berikutnya `kunci = { path = "..." }` (lokal)
///   ATAU `kunci = { git = "...", tag = "..." }` / `{ git = "...", rev = "..." }` (registry
///   git-based) sampai section lain (atau akhir berkas)
fn urai_manifest(isi: &str) -> Result<Manifest, String> {
    let mut m = Manifest::default();
    let mut di_dependensi = false;
    for (i, baris_mentah) in isi.lines().enumerate() {
        let baris = baris_mentah.trim();
        let no = i + 1;
        if baris.is_empty() || baris.starts_with('#') { continue; }
        if baris.starts_with('[') {
            if baris == "[dependensi]" { di_dependensi = true; continue; }
            return Err(format!("isoteri.toml baris {}: section \"{}\" tidak dikenal (cuma [dependensi] yang didukung di v1 ini).", no, baris));
        }
        let (kunci, nilai) = baris.split_once('=')
            .ok_or_else(|| format!("isoteri.toml baris {}: diharapkan \"kunci = nilai\", ditemukan \"{}\".", no, baris))?;
        let kunci = kunci.trim();
        let nilai = nilai.trim();
        if di_dependensi {
            // Format: nama = { path = "../lokasi" } ATAU nama = { git = "url", tag/rev = "..." }
            let dalam = nilai.strip_prefix('{').and_then(|s| s.strip_suffix('}'))
                .ok_or_else(|| format!("isoteri.toml baris {}: dependensi \"{}\" harus berbentuk {{ path = \"...\" }} atau {{ git = \"...\", tag = \"...\" }}.", no, kunci))?;
            let pasangan = urai_pasangan_kurung(dalam, no)?;
            let (mut path, mut git, mut tag, mut rev) = (None, None, None, None);
            for (k, v) in pasangan {
                match k.as_str() {
                    "path" => path = Some(v),
                    "git" => git = Some(v),
                    "tag" => tag = Some(v),
                    "rev" => rev = Some(v),
                    lain => return Err(format!("isoteri.toml baris {}: kunci \"{}\" tidak dikenal di dalam {{ }} (yang didukung: path, git, tag, rev).", no, lain)),
                }
            }
            let sumber = match (path, git) {
                (Some(p), None) => SumberDependensi::Lokal(p),
                (Some(_), Some(_)) => return Err(format!("isoteri.toml baris {}: dependensi \"{}\" tidak boleh punya \"path\" DAN \"git\" sekaligus.", no, kunci)),
                (None, Some(g)) => {
                    if tag.is_some() && rev.is_some() {
                        return Err(format!("isoteri.toml baris {}: dependensi \"{}\" tidak boleh punya \"tag\" DAN \"rev\" sekaligus -- pilih salah satu.", no, kunci));
                    }
                    if tag.is_none() && rev.is_none() {
                        return Err(format!("isoteri.toml baris {}: dependensi git \"{}\" butuh \"tag\" (rilis) atau \"rev\" (commit hash).", no, kunci));
                    }
                    SumberDependensi::Git { url: g, tag, rev }
                }
                (None, None) => return Err(format!("isoteri.toml baris {}: dependensi \"{}\" butuh \"path\" (lokal) atau \"git\" (registry).", no, kunci)),
            };
            m.dependensi.insert(kunci.to_string(), sumber);
        } else {
            let nilai_str = urai_string_toml(nilai)
                .ok_or_else(|| format!("isoteri.toml baris {}: nilai \"{}\" harus berupa Teks diapit tanda kutip dua.", no, nilai))?;
            match kunci {
                "nama" => m.nama = nilai_str,
                "versi" => m.versi = nilai_str,
                lain => return Err(format!("isoteri.toml baris {}: kunci \"{}\" tidak dikenal di level atas (yang didukung: nama, versi).", no, lain)),
            }
        }
    }
    if m.nama.is_empty() { return Err("isoteri.toml tidak punya \"nama\".".to_string()); }
    Ok(m)
}

fn urai_string_toml(s: &str) -> Option<String> {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') { Some(s[1..s.len() - 1].to_string()) } else { None }
}

pub fn baca_manifest(path: &std::path::Path) -> Result<Manifest, String> {
    let isi = fs::read_to_string(path).map_err(|e| format!("Tidak bisa membaca \"{}\": {}", path.display(), e))?;
    urai_manifest(&isi)
}

/// Cari `isoteri.toml` mulai dari `dir`, naik ke direktori induk berturut-turut sampai
/// ketemu atau mentok di root -- persis cara Cargo mencari Cargo.toml, supaya `muat "nama"`
/// tetap bisa meresolusi dependensi proyek dari submodul manapun di dalamnya, bukan cuma
/// dari direktori paling atas.
pub fn cari_manifest(dir: &std::path::Path) -> Option<(std::path::PathBuf, Manifest)> {
    let mut cur = Some(dir.to_path_buf());
    while let Some(d) = cur {
        let kandidat = d.join("isoteri.toml");
        if kandidat.is_file() {
            if let Ok(m) = baca_manifest(&kandidat) { return Some((kandidat, m)); }
            return None;
        }
        cur = d.parent().map(|p| p.to_path_buf());
    }
    None
}

/// Resolusi target `muat "..."`: path relatif biasa (perilaku lama, TIDAK berubah) kalau
/// berkasnya memang ada; kalau tidak DAN nama-nya "polos" (tanpa `/` atau akhiran `.iso` --
/// ciri nama paket, bukan path berkas), coba cari lewat manifest proyek -> `<path>/src/lib.iso`.
/// Mengembalikan path akhir yang dipakai (SELALU path relatif-biasa dulu kalau ada, supaya
/// proyek lama yang belum punya isoteri.toml sama sekali tidak terpengaruh apa pun).
fn resolusi_muat(rel_path: &str, dir_saat_ini: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let langsung = dir_saat_ini.join(rel_path);
    if langsung.exists() { return Ok(langsung); }

    let terlihat_seperti_nama_paket = !rel_path.contains('/') && !rel_path.contains('\\') && !rel_path.ends_with(".iso");
    if terlihat_seperti_nama_paket {
        if let Some((manifest_path, manifest)) = cari_manifest(dir_saat_ini) {
            if let Some(sumber) = manifest.dependensi.get(rel_path) {
                let root_proyek = manifest_path.parent().unwrap_or(std::path::Path::new("."));
                let paket_dir = match sumber {
                    SumberDependensi::Lokal(dep_path) => root_proyek.join(dep_path),
                    SumberDependensi::Git { url, tag, rev } =>
                        resolusi_paket_git(url, tag.as_deref(), rev.as_deref())?,
                };
                let target = paket_dir.join("src").join("lib.iso");
                if target.exists() { return Ok(target); }
                return Err(format!(
                    "'muat \"{}\"' ditemukan di [dependensi] isoteri.toml (mengarah ke \"{}\"), tapi \"{}\" tidak ada.",
                    rel_path, paket_dir.display(), target.display()
                ));
            }
        }
    }
    // Gagal semua cara -- kembalikan path langsung apa adanya, biar pesan error "tidak bisa
    // dibaca" di pemanggil tetap menunjuk ke path yang jelas & konsisten dengan perilaku lama.
    Ok(langsung)
}

// =====================================================================
// REGISTRY v1 -- GIT-BASED (lihat docs/FILOSOFI.md Milestone C)
// =====================================================================
//
// Keputusan arsitektur (dibahas & disepakati sebelum implementasi): registry TIDAK
// berbentuk server indeks terpusat (kayak npm/crates.io) yang harus dihosting & dijaga
// sendiri -- itu beban operasional besar yang belum perlu di tahap ini. Sebagai gantinya
// dipakai model Git-based (mirip Go modules / Deno): paket = repo Git APAPUN (GitHub,
// GitLab, Gitea, bahkan server Git pribadi) + satu tag rilis semver ATAU commit hash
// spesifik. `isoteri` tinggal `git clone` repo itu ke cache lokal, cek isinya sesuai
// konvensi paket yang SUDAH ada (`src/lib.iso`) -- tidak ada protokol/format baru yang
// perlu didesain dari nol.
//
// Kalau nanti ekosistem sudah ramai dan butuh DISCOVERY (search "paket matematika apa saja
// yang ada"), baru ditambah index server ringan (misal Cloudflare Worker, konsisten dengan
// infrastruktur ToFarmer yang sudah dipakai) yang cuma nyimpen metadata nama->URL repo,
// BUKAN hosting isi paketnya -- migrasi jadi mulus, tidak breaking dependensi yang sudah ada.

/// Direktori cache paket registry, default `~/.isoteri/cache` (bisa dioverride lewat env
/// `ISOTERI_CACHE_DIR`, berguna buat pengujian/CI supaya tidak menyentuh cache asli user).
fn direktori_cache() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("ISOTERI_CACHE_DIR") {
        return std::path::PathBuf::from(dir);
    }
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_else(|_| ".".to_string());
    std::path::Path::new(&home).join(".isoteri").join("cache")
}

/// Ganti tiap karakter yang bukan alfanumerik/`.`/`-` jadi `_`, supaya URL git & tag/rev
/// aman dipakai sebagai nama folder cache di semua OS.
fn sanitasi_nama_cache(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' }).collect()
}

/// Resolusi dependensi git: pastikan repo `url` pada `tag` (rilis) ATAU `rev` (commit hash)
/// sudah ada di cache lokal, lalu kembalikan direktori paketnya (siap dijoin dengan
/// `src/lib.iso` oleh pemanggil, sama seperti dependensi lokal).
///
/// CACHE = PIN: kalau folder cache buat kombinasi url+tag/rev ini sudah ada (ditandai folder
/// `.git` di dalamnya), TIDAK di-fetch ulang -- tag/rev dianggap penunjuk tetap ke isi yang
/// sama, sama seperti model cache Go modules. Konsekuensinya (didokumentasikan, bukan
/// disembunyikan, lihat docs/KETERBATASAN.md): kalau upstream memindahkan sebuah tag ke commit
/// lain (praktik buruk tapi mungkin terjadi), isoteri tidak akan otomatis mendeteksinya --
/// hapus manual folder cache-nya (atau pakai `rev` commit hash yang memang tidak bisa
/// dipindah) kalau itu terjadi.
pub fn resolusi_paket_git(url: &str, tag: Option<&str>, rev: Option<&str>) -> Result<std::path::PathBuf, String> {
    let versi_label = tag.or(rev).unwrap_or("HEAD");
    let target = direktori_cache().join(format!("{}-{}", sanitasi_nama_cache(url), sanitasi_nama_cache(versi_label)));

    if target.join(".git").is_dir() {
        return Ok(target); // sudah pernah diambil -- lihat catatan "CACHE = PIN" di atas
    }

    let root_cache = target.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| direktori_cache());
    fs::create_dir_all(&root_cache)
        .map_err(|e| format!("Tidak bisa membuat direktori cache \"{}\": {}", root_cache.display(), e))?;
    // Bersihkan sisa percobaan sebelumnya yang gagal di tengah jalan (folder ada tapi bukan
    // repo git valid) -- kalau tidak, `git clone` menolak folder tujuan yang sudah terisi.
    if target.exists() {
        let _ = fs::remove_dir_all(&target);
    }

    let target_str = target.to_string_lossy().to_string();
    let hasil_clone = if let Some(t) = tag {
        // --depth 1 + --branch juga menerima nama TAG (bukan cuma branch) di git, jadi ini
        // clone dangkal (cepat, hemat bandwidth) buat kasus paling umum: pin ke rilis.
        std::process::Command::new("git")
            .args(["clone", "--quiet", "--branch", t, "--depth", "1", url, &target_str])
            .status()
    } else {
        // Commit hash spesifik butuh histori penuh (server git kebanyakan tidak mendukung
        // shallow-fetch by-arbitrary-sha), jadi clone biasa lalu checkout persis rev-nya.
        std::process::Command::new("git")
            .args(["clone", "--quiet", url, &target_str])
            .status()
    };

    let status = match hasil_clone {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err("Perintah \"git\" tidak ditemukan di PATH -- instal git dulu buat memakai dependensi registry (git-based).".to_string());
        }
        Err(e) => return Err(format!("Gagal menjalankan git clone \"{}\": {}", url, e)),
    };
    if !status.success() {
        let _ = fs::remove_dir_all(&target);
        return Err(format!(
            "Gagal git clone \"{}\"{} -- cek URL, koneksi jaringan, dan (kalau pakai tag) apakah tag itu benar ada.",
            url, tag.map(|t| format!(" (tag \"{}\")", t)).unwrap_or_default()
        ));
    }

    if let Some(r) = rev {
        let status_checkout = std::process::Command::new("git")
            .args(["-C", &target_str, "checkout", "--quiet", r])
            .status();
        match status_checkout {
            Ok(s) if s.success() => {}
            Ok(_) => {
                let _ = fs::remove_dir_all(&target);
                return Err(format!("Gagal checkout rev \"{}\" di repo \"{}\" -- pastikan commit hash itu benar ada di repo ini.", r, url));
            }
            Err(e) => {
                let _ = fs::remove_dir_all(&target);
                return Err(format!("Gagal menjalankan git checkout: {}", e));
            }
        }
    }

    Ok(target)
}


//
// Filosofi: "formatter adalah sumber kebenaran gaya penulisan" -- bukan alat bantu
// opsional, tapi definisi TUNGGAL soal indentasi/spasi/tanda kurung, supaya tim/komunitas
// tidak perlu berdebat soal style (persis seperti gofmt/rustfmt/prettier). ini juga fondasi
// penting buat LSP/VS Code nanti ("format on save" butuh formatter yang sudah ada duluan).
//
// PENDEKATAN: cetak ulang dari AST (Stmt/Expr, BUKAN dari CStmt/CExpr yang sudah di-resolve
// -- itu sudah kehilangan nama variabel asli, diganti nomor slot), bukan sekadar
// menormalkan whitespace dari teks sumber apa adanya. Ini "sumber kebenaran" yang sesungguhnya:
// hasilnya SELALU konsisten terlepas dari gaya penulisan asli, dan idempoten (format ulang
// hasil yang sudah diformat = tidak berubah lagi, lihat verifikasi di test).
//
// TANTANGAN UTAMA: parser & lexer PRODUKSI (dipakai compiler) membuang komentar
// (`catatan: ...`) sepenuhnya, tidak disimpan di AST sama sekali (lihat Lexer::tokenize).
// Formatter yang naif cetak-ulang-dari-AST akan DIAM-DIAM MENGHAPUS SEMUA KOMENTAR --
// itu BUG SERIUS buat formatter sungguhan (rustfmt/prettier tidak pernah begitu).
//
// Solusinya SENGAJA tidak mengubah Lexer/Parser produksi sama sekali (nol risiko regresi ke
// compiler): `Lexer::tokenize_dengan_komentar()` (method BARU, terpisah) menghasilkan token
// stream yang MASIH menyertakan komentar sebagai `Token::Komentar`. Formatter menyaring baris
// komentarnya jadi peta baris->teks, MEMBUANG token komentar itu dari stream, lalu memberi
// sisanya ke `Parser::new()` YANG SAMA PERSIS dipakai compiler (tidak diubah sedikit pun) --
// jadi AST yang dicetak ulang dijamin PERSIS merepresentasikan makna program yang sama.
// Komentar ditempelkan kembali berdasarkan nomor barisnya sebelum tiap statement yang jatuh
// setelahnya.
//
// KETERBATASAN v1 (didokumentasikan, bukan disembunyikan -- Hukum "jangan diam-diam
// merusak/kehilangan isi" lebih penting daripada cakupan lengkap):
//   - Komentar HARUS di baris sendiri. Komentar di baris yang SAMA dengan kode
//     (`tampilkan x  catatan: ini penjelasan`) DITOLAK dengan pesan jelas (bukan
//     diam-diam dibuang atau ditaruh di posisi salah).
//   - Komentar yang jadi baris PALING TERAKHIR di dalam sebuah blok bersarang (tepat
//     sebelum '}' penutup, dengan tidak ada apa pun lagi setelahnya di SISA program)
//     akan muncul di akhir keluaran, bukan tepat sebelum '}' itu -- kasus tepi yang jarang,
//     tapi komentarnya tetap ADA (tidak hilang), cuma posisinya mungkin tidak persis.

const INDENT_FORMAT: &str = "    "; // 4 spasi -- konsisten dgn semua program*.iso di proyek ini

fn prec_binop(op: BinOp) -> u8 {
    use BinOp::*;
    match op {
        Atau => 1,
        Dan => 2,
        SamaDengan | TidakSama => 3,
        LebihBesar | LebihBesarSama | LebihKecil | LebihKecilSama => 4,
        Tambah | Kurang => 5,
        Kali | Bagi | Modulo => 6,
    }
}
fn str_binop(op: BinOp) -> &'static str {
    use BinOp::*;
    match op {
        Tambah => "+", Kurang => "-", Kali => "*", Bagi => "/", Modulo => "%",
        SamaDengan => "==", TidakSama => "!=",
        LebihBesar => ">", LebihBesarSama => ">=", LebihKecil => "<", LebihKecilSama => "<=",
        Dan => "dan", Atau => "atau",
    }
}

fn escape_teks_format(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            other => o.push(other),
        }
    }
    o
}

fn format_desimal_literal(f: f64) -> String {
    let s = format!("{}", f);
    if s.contains('.') || s.contains('e') || s.contains("inf") || s.contains("NaN") { s } else { format!("{}.0", s) }
}

fn cetak_daftar_parameter_fmt(params: &[(String, Option<String>)]) -> String {
    params.iter().map(|(n, t)| match t { Some(t) => format!("{}: {}", n, t), None => n.clone() }).collect::<Vec<_>>().join(", ")
}

/// `Expr::Binary(Angka(0), Kurang, x)` adalah bentuk desugar dari unary minus `-x` (lihat
/// Parser::parse_unary) -- dikenali di sini supaya dicetak balik sebagai `-x`, bukan `0 - x`.
fn sebagai_unary_neg(e: &Expr) -> Option<&Expr> {
    if let Expr::Binary(l, BinOp::Kurang, r) = e {
        if let Expr::Angka(0) = **l { return Some(r); }
    }
    None
}

type PetaKomentar = HashMap<usize, Vec<String>>;

fn flush_komentar_sampai(batas_baris: usize, indent: usize, komentar: &PetaKomentar, terpakai: &mut std::collections::HashSet<usize>, out: &mut String) {
    let mut baris_terurut: Vec<usize> = komentar.keys().copied().filter(|b| *b <= batas_baris && !terpakai.contains(b)).collect();
    baris_terurut.sort_unstable();
    for b in baris_terurut {
        for teks in &komentar[&b] {
            out.push_str(&INDENT_FORMAT.repeat(indent));
            out.push_str(teks);
            out.push('\n');
        }
        terpakai.insert(b);
    }
}

fn cetak_expr_fmt(e: &Expr, indent: usize, min_prec: u8, komentar: &PetaKomentar, terpakai: &mut std::collections::HashSet<usize>) -> String {
    if let Some(inner) = sebagai_unary_neg(e) {
        return format!("-{}", cetak_expr_fmt(inner, indent, 7, komentar, terpakai));
    }
    match e {
        Expr::Angka(n) => n.to_string(),
        Expr::Desimal(f) => format_desimal_literal(*f),
        Expr::Teks(s) => format!("\"{}\"", escape_teks_format(s)),
        Expr::Bool(b) => (if *b { "benar" } else { "salah" }).to_string(),
        Expr::Ident(s) => s.clone(),
        Expr::Tidak(inner) => format!("!{}", cetak_expr_fmt(inner, indent, 7, komentar, terpakai)),
        Expr::Binary(l, op, r) => {
            let p = prec_binop(*op);
            let teks = format!(
                "{} {} {}",
                cetak_expr_fmt(l, indent, p, komentar, terpakai),
                str_binop(*op),
                cetak_expr_fmt(r, indent, p + 1, komentar, terpakai),
            );
            if p < min_prec { format!("({})", teks) } else { teks }
        }
        Expr::Panggil(nama, args) => format!("{}({})", nama, args.iter().map(|a| cetak_expr_fmt(a, indent, 0, komentar, terpakai)).collect::<Vec<_>>().join(", ")),
        Expr::Daftar(items) => format!("[{}]", items.iter().map(|a| cetak_expr_fmt(a, indent, 0, komentar, terpakai)).collect::<Vec<_>>().join(", ")),
        Expr::Peta(entries) => format!(
            "{{{}}}",
            entries.iter().map(|(k, v)| format!("\"{}\": {}", escape_teks_format(k), cetak_expr_fmt(v, indent, 0, komentar, terpakai))).collect::<Vec<_>>().join(", ")
        ),
        Expr::Indeks(t, i) => format!("{}[{}]", cetak_expr_fmt(t, indent, 8, komentar, terpakai), cetak_expr_fmt(i, indent, 0, komentar, terpakai)),
        Expr::Field(t, f) => format!("{}.{}", cetak_expr_fmt(t, indent, 8, komentar, terpakai), f),
        Expr::PanggilMetode(t, f, args) => format!("{}.{}({})", cetak_expr_fmt(t, indent, 8, komentar, terpakai), f, args.iter().map(|a| cetak_expr_fmt(a, indent, 0, komentar, terpakai)).collect::<Vec<_>>().join(", ")),
        Expr::BentukLiteral(nama, entries) => {
            if entries.is_empty() { format!("{} {{}}", nama) }
            else {
                format!(
                    "{} {{ {} }}",
                    nama,
                    entries.iter().map(|(k, v)| format!("{}: {}", k, cetak_expr_fmt(v, indent, 0, komentar, terpakai))).collect::<Vec<_>>().join(", ")
                )
            }
        }
        Expr::FungsiLiteral(params, body) => {
            let mut s = format!("fungsi({}) {{\n", cetak_daftar_parameter_fmt(params));
            cetak_blok_fmt(body, indent + 1, komentar, terpakai, &mut s);
            s.push_str(&INDENT_FORMAT.repeat(indent));
            s.push('}');
            s
        }
    }
}

fn cetak_stmt_fmt(s: &Stmt, indent: usize, komentar: &PetaKomentar, terpakai: &mut std::collections::HashSet<usize>, out: &mut String) {
    match s {
        Stmt::Ingat(nama, tipe, e) => {
            out.push_str("ingat ");
            out.push_str(nama);
            if let Some(t) = tipe { out.push_str(": "); out.push_str(t); }
            out.push_str(" = ");
            out.push_str(&cetak_expr_fmt(e, indent, 0, komentar, terpakai));
        }
        Stmt::Ubah(nama, e) => {
            out.push_str(nama);
            out.push_str(" = ");
            out.push_str(&cetak_expr_fmt(e, indent, 0, komentar, terpakai));
        }
        Stmt::UbahField(nama, fields, e) => {
            out.push_str(nama);
            for f in fields { out.push('.'); out.push_str(f); }
            out.push_str(" = ");
            out.push_str(&cetak_expr_fmt(e, indent, 0, komentar, terpakai));
        }
        Stmt::UbahJalur(nama, jalur, e) => {
            out.push_str(nama);
            for j in jalur {
                match j {
                    Jalur::Field(f) => { out.push('.'); out.push_str(f); }
                    Jalur::Indeks(idx) => {
                        out.push('[');
                        out.push_str(&cetak_expr_fmt(idx, indent, 0, komentar, terpakai));
                        out.push(']');
                    }
                }
            }
            out.push_str(" = ");
            out.push_str(&cetak_expr_fmt(e, indent, 0, komentar, terpakai));
        }
        Stmt::BentukDef(nama, fields) => {
            out.push_str("bentuk ");
            out.push_str(nama);
            out.push_str(" {\n");
            for (i, (fnama, ftipe)) in fields.iter().enumerate() {
                out.push_str(&INDENT_FORMAT.repeat(indent + 1));
                out.push_str(fnama);
                if let Some(t) = ftipe { out.push_str(": "); out.push_str(t); }
                if i + 1 < fields.len() { out.push(','); }
                out.push('\n');
            }
            out.push_str(&INDENT_FORMAT.repeat(indent));
            out.push('}');
        }
        Stmt::Muat(path, alias) => {
            out.push_str("muat \"");
            out.push_str(&escape_teks_format(path));
            out.push('"');
            if let Some(a) = alias { out.push_str(" sebagai "); out.push_str(a); }
        }
        Stmt::Tampilkan(e) => { out.push_str("tampilkan "); out.push_str(&cetak_expr_fmt(e, indent, 0, komentar, terpakai)); }
        Stmt::Kalau(cond, tb, eb) => {
            out.push_str("kalau (");
            out.push_str(&cetak_expr_fmt(cond, indent, 0, komentar, terpakai));
            out.push_str(") {\n");
            cetak_blok_fmt(tb, indent + 1, komentar, terpakai, out);
            out.push_str(&INDENT_FORMAT.repeat(indent));
            out.push('}');
            if let Some(eb) = eb {
                // Rantai 'lainnya kalau' didesugar parser jadi blok satu-statement berisi
                // Stmt::Kalau lagi (lihat parse_stmt) -- deteksi pola itu di sini supaya
                // formatter mencetaknya balik sebagai 'lainnya kalau (...)', bukan
                // 'lainnya { kalau (...) { ... } }' yang secara makna sama tapi bukan gaya asli.
                if let [(_, Stmt::Kalau(..))] = eb.as_slice() {
                    out.push_str(" lainnya ");
                    cetak_stmt_fmt(&eb[0].1, indent, komentar, terpakai, out);
                } else {
                    out.push_str(" lainnya {\n");
                    cetak_blok_fmt(eb, indent + 1, komentar, terpakai, out);
                    out.push_str(&INDENT_FORMAT.repeat(indent));
                    out.push('}');
                }
            }
        }
        Stmt::Ulang(cond, body) => {
            out.push_str("ulang (");
            out.push_str(&cetak_expr_fmt(cond, indent, 0, komentar, terpakai));
            out.push_str(") {\n");
            cetak_blok_fmt(body, indent + 1, komentar, terpakai, out);
            out.push_str(&INDENT_FORMAT.repeat(indent));
            out.push('}');
        }
        Stmt::UlangSetiap(var, e, body) => {
            out.push_str("ulang setiap ");
            out.push_str(var);
            out.push_str(" dari ");
            out.push_str(&cetak_expr_fmt(e, indent, 0, komentar, terpakai));
            out.push_str(" {\n");
            cetak_blok_fmt(body, indent + 1, komentar, terpakai, out);
            out.push_str(&INDENT_FORMAT.repeat(indent));
            out.push('}');
        }
        Stmt::UlangSelaras(var, e, body) => {
            out.push_str("ulang selaras setiap ");
            out.push_str(var);
            out.push_str(" dari ");
            out.push_str(&cetak_expr_fmt(e, indent, 0, komentar, terpakai));
            out.push_str(" {\n");
            cetak_blok_fmt(body, indent + 1, komentar, terpakai, out);
            out.push_str(&INDENT_FORMAT.repeat(indent));
            out.push('}');
        }
        Stmt::FungsiDef(nama, params, body) => {
            out.push_str("fungsi ");
            out.push_str(nama);
            out.push('(');
            out.push_str(&cetak_daftar_parameter_fmt(params));
            out.push_str(") {\n");
            cetak_blok_fmt(body, indent + 1, komentar, terpakai, out);
            out.push_str(&INDENT_FORMAT.repeat(indent));
            out.push('}');
        }
        Stmt::Kembalikan(e) => { out.push_str("kembalikan "); out.push_str(&cetak_expr_fmt(e, indent, 0, komentar, terpakai)); }
        Stmt::EkspresiStmt(e) => out.push_str(&cetak_expr_fmt(e, indent, 0, komentar, terpakai)),
        Stmt::Putus => out.push_str("putus"),
        Stmt::Lanjut => out.push_str("lanjut"),
        Stmt::Coba(bc, var, bt) => {
            out.push_str("coba {\n");
            cetak_blok_fmt(bc, indent + 1, komentar, terpakai, out);
            out.push_str(&INDENT_FORMAT.repeat(indent));
            out.push_str("} tangkap ");
            out.push_str(var);
            out.push_str(" {\n");
            cetak_blok_fmt(bt, indent + 1, komentar, terpakai, out);
            out.push_str(&INDENT_FORMAT.repeat(indent));
            out.push('}');
        }
    }
}

fn cetak_blok_fmt(stmts: &[(usize, Stmt)], indent: usize, komentar: &PetaKomentar, terpakai: &mut std::collections::HashSet<usize>, out: &mut String) {
    for (baris, s) in stmts {
        flush_komentar_sampai(*baris, indent, komentar, terpakai, out);
        out.push_str(&INDENT_FORMAT.repeat(indent));
        cetak_stmt_fmt(s, indent, komentar, terpakai, out);
        out.push('\n');
    }
}

/// Cetak Vec<Stmt> (hasil ekspansi_muat + rewrite alias, BUKAN dari lexer/parser mentah)
/// balik jadi teks source Isoteri -- TANPA komentar asli (sudah hilang sejak parsing, wajar,
/// bukan bug -- AST tidak menyimpan komentar). Dipakai `isoteri bangun` (mode_bangun di
/// main.rs): setelah semua 'muat' (termasuk alias) diproses lewat program_dari_berkas(), hasil
/// Vec<Stmt>-nya perlu ditempel sebagai SATU string literal ke crate Rust sementara yang
/// di-generate -- makanya perlu dicetak balik jadi teks, bukan dikirim sebagai AST langsung.
pub fn cetak_program_ke_sumber(program: &[(usize, Stmt)]) -> String {
    let komentar_kosong: PetaKomentar = std::collections::HashMap::new();
    let mut terpakai = std::collections::HashSet::new();
    let mut out = String::new();
    cetak_blok_fmt(program, 0, &komentar_kosong, &mut terpakai, &mut out);
    out
}

/// Format satu berkas sumber Isoteri (SATU berkas -- `muat` TIDAK diekspansi, beda dari
/// kompilasi; format satu berkas seharusnya tidak menarik masuk isi berkas lain).
pub fn format_sumber(sumber: &str) -> Result<String, String> {
    let mut lexer = Lexer::new(sumber);
    let token_dgn_komentar = lexer.tokenize_dengan_komentar().map_err(|e| format!("Kesalahan Lexer: {}", e))?;

    let mut baris_ada_kode: std::collections::HashSet<usize> = std::collections::HashSet::new();
    for (t, b) in &token_dgn_komentar {
        if !matches!(t, Token::Komentar(_)) && *t != Token::Eof { baris_ada_kode.insert(*b); }
    }
    let mut komentar: PetaKomentar = HashMap::new();
    for (t, b) in &token_dgn_komentar {
        if let Token::Komentar(teks) = t {
            if baris_ada_kode.contains(b) {
                return Err(format!(
                    "Baris {}: formatter belum mendukung komentar di baris yang sama dengan kode (\"{}\") -- pindahkan ke baris tersendiri dulu.",
                    b, teks
                ));
            }
            komentar.entry(*b).or_default().push(teks.clone());
        }
    }

    let token_bersih: Vec<(Token, usize)> = token_dgn_komentar.into_iter().filter(|(t, _)| !matches!(t, Token::Komentar(_))).collect();
    let mut parser = Parser::new(token_bersih);
    let program = parser.parse_program().map_err(|e| format!("Kesalahan Parser: {}", e))?;

    let mut out = String::new();
    let mut terpakai = std::collections::HashSet::new();
    cetak_blok_fmt(&program, 0, &komentar, &mut terpakai, &mut out);
    // Komentar yang belum "terpakai" (mis. baris terakhir berkas, atau -- lihat keterbatasan
    // v1 di atas -- baris terakhir sebuah blok bersarang) tetap DITULISKAN di sini, supaya
    // TIDAK PERNAH hilang diam-diam walau posisinya mungkin bukan pas di tempat asalnya.
    flush_komentar_sampai(usize::MAX, 0, &komentar, &mut terpakai, &mut out);
    Ok(out)
}

pub fn format_berkas(path: &str) -> Result<String, String> {
    let sumber = fs::read_to_string(path).map_err(|e| format!("Tidak bisa membaca \"{}\": {}", path, e))?;
    format_sumber(&sumber)
}

/// muat modul lain relatif ke posisinya sendiri. `sudah_dimuat` mencegah muat ganda/siklus
/// (mirip include guard di C) -- file yang sama cuma pernah diekspansi sekali.
/// Tiap statement hasil ekspansi dibawa bareng LABEL file asalnya (dipakai belakangan oleh
/// cek_tabrakan_nama buat mendeteksi dua modul beda yang kebetulan pakai nama sama).
fn ekspansi_muat(
    stmts: Vec<(usize, Stmt)>,    label_saat_ini: &str,
    dir_saat_ini: &std::path::Path,
    sudah_dimuat: &mut std::collections::HashSet<std::path::PathBuf>,
    alias_dikenal: &mut std::collections::HashSet<String>,
) -> Result<Vec<(usize, Stmt, Rc<str>)>, String> {
    let mut hasil = Vec::with_capacity(stmts.len());
    for (baris, s) in stmts {
        if let Stmt::Muat(rel_path, alias) = &s {
            let target = resolusi_muat(rel_path, dir_saat_ini)?;
            let target_kanonik = fs::canonicalize(&target).unwrap_or_else(|_| target.clone());
            if sudah_dimuat.contains(&target_kanonik) {
                continue; // sudah pernah dimuat -- lewati diam-diam (include guard)
            }
            sudah_dimuat.insert(target_kanonik);

            let sumber = fs::read_to_string(&target)
                .map_err(|e| format!("Baris {}: Tidak bisa memuat \"{}\": {}", baris, rel_path, e))?;
            let mut lexer = Lexer::new(&sumber);
            let tokens = lexer.tokenize().map_err(|e| format!("[{}] Kesalahan Lexer: {}", target.display(), e))?;
            let mut parser = Parser::new(tokens);
            let sub_stmts = parser.parse_program().map_err(|e| format!("[{}] Kesalahan Parser: {}", target.display(), e))?;

            let sub_dir = target.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| std::path::PathBuf::from("."));
            let label_target = target.display().to_string();
            let sub_hasil = ekspansi_muat(sub_stmts, &label_target, &sub_dir, sudah_dimuat, alias_dikenal)?;

            match alias {
                None => hasil.extend(sub_hasil), // perilaku lama: flat, numplek ke namespace global
                Some(alias_nama) => {
                    if !alias_dikenal.insert(alias_nama.clone()) {
                        return Err(format!("Baris {}: alias modul \"{}\" sudah dipakai sebelumnya -- pakai nama alias lain.", baris, alias_nama));
                    }
                    hasil.extend(mangle_modul_beralias(sub_hasil, alias_nama));
                }
            }
        } else {
            hasil.push((baris, s, Rc::from(label_saat_ini)));
        }
    }
    Ok(hasil)
}

/// Ganti nama semua fungsi TOP-LEVEL yang dideklarasikan langsung di modul `sub_hasil` (bukan
/// yang transitif lewat 'muat' TANPA alias di dalamnya -- itu ikut ke-mangle juga karena sudah
/// tercampur flat di sub_hasil, itu memang disengaja: apapun yang masuk lewat 'muat X sebagai a'
/// -- baik langsung atau transitif tanpa alias sendiri -- jadi bagian namespace 'a'). Modul yang
/// SUDAH punya nama ter-mangle sendiri (dari 'muat Y sebagai b' bersarang di dalam X) TIDAK
/// di-mangle ulang -- namanya (mis. "b.fungsi") tetap apa adanya, cuma bisa diakses lewat
/// "b.fungsi" dari MANA PUN (termasuk dari luar alias 'a') -- keterbatasan yang didokumentasikan,
/// lihat KETERBATASAN.md.
fn mangle_modul_beralias(sub_hasil: Vec<(usize, Stmt, Rc<str>)>, alias: &str) -> Vec<(usize, Stmt, Rc<str>)> {
    let nama_fungsi_lokal: std::collections::HashSet<String> = sub_hasil.iter()
        .filter_map(|(_, s, _)| match s {
            Stmt::FungsiDef(nama, ..) if !nama.starts_with("__modul_") => Some(nama.clone()),
            _ => None,
        })
        .collect();
    if nama_fungsi_lokal.is_empty() { return sub_hasil; }
    sub_hasil.into_iter().map(|(baris, s, label)| {
        (baris, mangle_stmt(s, alias, &nama_fungsi_lokal), label)
    }).collect()
}

fn mangle_nama_jika_perlu(nama: &str, alias: &str, set: &std::collections::HashSet<String>) -> String {
    // Separator "__modul_X__Y" (BUKAN titik) SENGAJA dipilih: harus tetap jadi SATU token
    // identifier yang sah kalau di-lex ULANG dari teks -- ini penting karena `isoteri bangun`
    // (AOT) nge-roundtrip AST modul beralias balik ke TEKS lewat cetak_program_ke_sumber(), lalu
    // teks itu di-lex+parse ULANG dari nol saat binary hasil bangun-nya dijalankan. Titik ('.')
    // dulu dicoba duluan tapi GAGAL roundtrip: lexer selalu pecah "a.b" jadi 3 token terpisah
    // (Identifikator, Titik, Identifikator), bukan 1 identifier utuh, walau di jalur bytecode/
    // JSON biasa (yang gak pernah nge-lex ulang nama fungsi) itu nggak masalah.
    if set.contains(nama) { format!("__modul_{}__{}", alias, nama) } else { nama.to_string() }
}

fn mangle_stmt(s: Stmt, alias: &str, set: &std::collections::HashSet<String>) -> Stmt {
    match s {
        Stmt::FungsiDef(nama, params, body) => {
            let nama_baru = mangle_nama_jika_perlu(&nama, alias, set);
            let body_baru = body.into_iter().map(|(b, st)| (b, mangle_stmt(st, alias, set))).collect();
            Stmt::FungsiDef(nama_baru, params, body_baru)
        }
        Stmt::Kalau(c, tb, eb) => Stmt::Kalau(
            mangle_expr(c, alias, set),
            tb.into_iter().map(|(b, st)| (b, mangle_stmt(st, alias, set))).collect(),
            eb.map(|blk| blk.into_iter().map(|(b, st)| (b, mangle_stmt(st, alias, set))).collect()),
        ),
        Stmt::Ulang(c, body) => Stmt::Ulang(mangle_expr(c, alias, set), body.into_iter().map(|(b, st)| (b, mangle_stmt(st, alias, set))).collect()),
        Stmt::UlangSetiap(v, e, body) => Stmt::UlangSetiap(v, mangle_expr(e, alias, set), body.into_iter().map(|(b, st)| (b, mangle_stmt(st, alias, set))).collect()),
        Stmt::UlangSelaras(v, e, body) => Stmt::UlangSelaras(v, mangle_expr(e, alias, set), body.into_iter().map(|(b, st)| (b, mangle_stmt(st, alias, set))).collect()),
        Stmt::Coba(tb, v, cb) => Stmt::Coba(
            tb.into_iter().map(|(b, st)| (b, mangle_stmt(st, alias, set))).collect(), v,
            cb.into_iter().map(|(b, st)| (b, mangle_stmt(st, alias, set))).collect(),
        ),
        Stmt::Ingat(nama, tipe, e) => Stmt::Ingat(nama, tipe, mangle_expr(e, alias, set)),
        Stmt::Ubah(nama, e) => Stmt::Ubah(nama, mangle_expr(e, alias, set)),
        Stmt::UbahField(nama, jalur, e) => Stmt::UbahField(nama, jalur, mangle_expr(e, alias, set)),
        Stmt::UbahJalur(nama, jalur, e) => Stmt::UbahJalur(nama, jalur.into_iter().map(|j| match j {
            Jalur::Indeks(ie) => Jalur::Indeks(mangle_expr(ie, alias, set)),
            lain => lain,
        }).collect(), mangle_expr(e, alias, set)),
        Stmt::Tampilkan(e) => Stmt::Tampilkan(mangle_expr(e, alias, set)),
        Stmt::Kembalikan(e) => Stmt::Kembalikan(mangle_expr(e, alias, set)),
        Stmt::EkspresiStmt(e) => Stmt::EkspresiStmt(mangle_expr(e, alias, set)),
        lain => lain, // BentukDef, Muat, Putus, Lanjut -- tidak ada Expr/nama fungsi di dalamnya
    }
}

fn mangle_expr(e: Expr, alias: &str, set: &std::collections::HashSet<String>) -> Expr {
    match e {
        Expr::Panggil(nama, args) => {
            let nama_baru = mangle_nama_jika_perlu(&nama, alias, set);
            Expr::Panggil(nama_baru, args.into_iter().map(|a| mangle_expr(a, alias, set)).collect())
        }
        Expr::Binary(l, op, r) => Expr::Binary(Box::new(mangle_expr(*l, alias, set)), op, Box::new(mangle_expr(*r, alias, set))),
        Expr::Daftar(items) => Expr::Daftar(items.into_iter().map(|i| mangle_expr(i, alias, set)).collect()),
        Expr::Peta(entries) => Expr::Peta(entries.into_iter().map(|(k, v)| (k, mangle_expr(v, alias, set))).collect()),
        Expr::Indeks(t, i) => Expr::Indeks(Box::new(mangle_expr(*t, alias, set)), Box::new(mangle_expr(*i, alias, set))),
        Expr::Field(t, f) => Expr::Field(Box::new(mangle_expr(*t, alias, set)), f),
        Expr::PanggilMetode(t, f, args) => Expr::PanggilMetode(Box::new(mangle_expr(*t, alias, set)), f, args.into_iter().map(|a| mangle_expr(a, alias, set)).collect()),
        Expr::Tidak(e) => Expr::Tidak(Box::new(mangle_expr(*e, alias, set))),
        Expr::BentukLiteral(nama, entries) => Expr::BentukLiteral(nama, entries.into_iter().map(|(k, v)| (k, mangle_expr(v, alias, set))).collect()),
        Expr::FungsiLiteral(params, body) => Expr::FungsiLiteral(params, body.into_iter().map(|(b, st)| (b, mangle_stmt(st, alias, set))).collect()),
        lain => lain, // Angka, Desimal, Teks, Bool, Ident -- tidak ada nama fungsi di dalamnya
    }
}

/// Deteksi nama (fungsi/bentuk/variabel global) yang dideklarasikan di LEBIH DARI SATU file
/// berbeda hasil 'muat' -- sebelumnya ini gagal diam-diam (yang belakangan menang, nimpa yang
/// sebelumnya, tanpa peringatan apapun). Sengaja HANYA cek lintas-file: dua deklarasi nama sama
/// di FILE YANG SAMA tetap dibiarkan (perilaku lama, di luar cakupan perbaikan modul ini).
fn cek_tabrakan_nama(stmts: &[(usize, Stmt, Rc<str>)]) -> Result<(), String> {
    let mut asal: HashMap<String, (Rc<str>, &'static str)> = HashMap::new();
    for (_, s, label) in stmts {
        let (nama, jenis) = match s {
            Stmt::FungsiDef(nama, ..) => (nama, "fungsi"),
            Stmt::BentukDef(nama, ..) => (nama, "bentuk"),
            Stmt::Ingat(nama, ..) => (nama, "variabel global"),
            _ => continue,
        };
        match asal.get(nama) {
            Some((label_lama, jenis_lama)) if label_lama.as_ref() != label.as_ref() => {
                return Err(format!(
                    "Nama \"{}\" dideklarasikan di dua modul berbeda: {} \"{}\" di [{}], dan {} \"{}\" di [{}]. Ganti salah satu nama supaya gak tabrakan.",
                    nama, jenis_lama, nama, label_lama, jenis, nama, label
                ));
            }
            _ => { asal.insert(nama.clone(), (label.clone(), jenis)); }
        }
    }
    Ok(())
}

// kumpulkan_sumber_gabungan/kumpulkan_rekursif (implementasi TEKSTUAL lama buat gabungan
// 'muat' -- tanpa alias) sudah DIHAPUS, digantikan program_dari_berkas() yang berbasis AST
// (paham 'sebagai alias') dan dipakai SEMUA entry point sekarang -- lihat komentar
// program_dari_berkas() untuk kenapa dua implementasi terpisah itu berbahaya.

/// diekspansi lagi -- entah karena memang gak pakai modul, atau karena sudah diekspansi/
/// digabung sebelumnya, mis. oleh bundler AOT lewat `bangun_bundel`). Dipakai baik oleh CLI
/// biasa (via jalankan_berkas) maupun oleh binary hasil bundling AOT.
/// Cuma validasi (lexer+parser+resolver+compiler) TANPA menjalankan programnya -- dipakai
/// subcommand `bangun` (AOT, lihat main.rs) buat kasih feedback cepat kalau ada error bahasa
/// Isoteri, sebelum buang waktu manggil `cargo build` yang jauh lebih lambat & pesan errornya
/// (kalau ada) akan bercampur dengan error Rust yang membingungkan.
pub fn periksa_sumber(sumber: &str) -> Result<(), String> {
    let mut lexer = Lexer::new(sumber);
    let tokens = lexer.tokenize().map_err(|e| format!("Kesalahan Lexer: {}", e))?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().map_err(|e| format!("Kesalahan Parser: {}", e))?;
    let mut resolver = Resolver::new();
    let top_level = resolver.resolve_top(&program).map_err(|e| format!("Kesalahan Kompilasi: {}", e))?;

    let mut nama_fungsi: Vec<String> = resolver.fungsi_out.keys().cloned().collect();
    nama_fungsi.sort(); // deterministik: HashMap tidak menjamin urutan iterasi konsisten antar-run,
    // jadi kalau tidak diurutkan, urutan fungsi (dan index-nya) di bundel .isoweb.json bisa
    // berbeda-beda tiap kali di-compile ulang walau source-nya sama persis (nondeterminism
    // bawaan, ditemukan sewaktu verifikasi representasi Daftar flat -- lihat ROADMAP.md).
    let fungsi_index: HashMap<String, usize> = nama_fungsi.iter().enumerate().map(|(i, n)| (n.clone(), i)).collect();
    let mut compiler = Compiler::new(fungsi_index);
    let _ = compiler.compile_top(&top_level);
    for nama in &nama_fungsi {
        let cf = resolver.fungsi_out.get(nama).unwrap();
        let _ = compiler.compile_fungsi(cf);
    }
    Ok(())
}

pub fn jalankan_sumber(sumber: &str) -> Result<(), String> {
    let mut lexer = Lexer::new(sumber);
    let tokens = lexer.tokenize().map_err(|e| format!("Kesalahan Lexer: {}", e))?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().map_err(|e| format!("Kesalahan Parser: {}", e))?;
    jalankan_stmt_list(program)
}

/// Padanan `jalankan_sumber`, tapi lewat pipeline IR LINEAR (bagian "8b") -- dipakai
/// `isoteri bangun` (AOT, lihat main.rs `mode_bangun`) supaya binary hasil build jalan
/// lewat backend bytecode+JIT yang sama-sama generate dari IR, bukan menelusuri
/// CExpr/CStmt langsung seperti `jalankan_sumber` (jalur lama). Migrasi ini AMAN dipakai
/// buat AOT (blast radius kecil kalau ada apa-apa -- binary AOT sepenuhnya terpisah
/// proses dari CLI utama) dan sudah divalidasi ketat lewat `isoteri via-ir` (regresi
/// 17/17 + kasus rekursif JIT, lihat docs/IR.md) sebelum dipakai di sini.
pub fn jalankan_sumber_via_ir(sumber: &str) -> Result<(), String> {
    let mut lexer = Lexer::new(sumber);
    let tokens = lexer.tokenize().map_err(|e| format!("Kesalahan Lexer: {}", e))?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().map_err(|e| format!("Kesalahan Parser: {}", e))?;
    jalankan_stmt_list_via_ir(program)
}

/// Jalankan Isoteri mulai dari sebuah berkas `.iso` di disk -- termasuk mengekspansi semua
/// 'muat' yang ditulis di dalamnya (lihat ekspansi_muat) dan mengecek tabrakan nama lintas
/// modul (lihat cek_tabrakan_nama). Ini yang dipakai CLI normal (`isoteri program.iso`).
pub fn jalankan_berkas(path: &str) -> Result<(), String> {
    if !std::path::Path::new(path).exists() {
        eprintln!("Peringatan: tidak bisa membaca '{}', pakai skrip contoh bawaan.", path);
        let sumber = "ingat nilai = 80\nkalau (nilai >= 75) {\n    tampilkan \"Lulus\"\n} lainnya {\n    tampilkan \"Tidak lulus\"\n}";
        return jalankan_sumber(sumber);
    }
    jalankan_stmt_list(program_dari_berkas(path)?)
}

/// Inti pipeline resolve -> compile -> JIT -> eksekusi, dipakai bersama oleh jalankan_sumber
/// dan jalankan_berkas setelah keduanya menyiapkan Vec<Stmt> yang siap diresolve.
// Helper kecil: bungkus panggilan jit.kompilasi() + transmute jadi NativeFn dalam satu
// tempat -- dipakai jalankan_stmt_list & (via helper serupa) jalankan_stmt_list_via_ir.
// Di-cfg fitur "jit" supaya build tanpa Cranelift (mis. isoteri-wasm/) tidak perlu tahu
// apa-apa soal transmute pointer JIT sama sekali.
#[cfg(feature = "jit")]
fn coba_kompilasi_jit(jit: &mut JitEngine, cf: &CFungsi, mode: TipeJit) -> Result<NativeFn, String> {
    // Mode Campur (field/param tipe campuran, lihat catatan panjang di enum TipeJit) SENGAJA
    // cuma didukung di jalur kompilasi_dari_ir/via-ir/AOT (kompilasi_dari_ir), BUKAN di jalur
    // legacy CExpr-langsung ini (dipakai runtime JIT default `isoteri prog.iso`) -- refuse di
    // sini, fallback ke interpreter (aman, correctness tidak terpengaruh, cuma jalur ini yang
    // tidak dapat manfaat native compile buat fungsi Campur; AOT tetap dapat).
    if mode == TipeJit::Campur {
        return Err("mode Campur belum didukung di jalur JIT legacy (dipakai jalur IR/AOT saja)".to_string());
    }
    let ptr = jit.kompilasi(cf, mode)?;
    Ok(match mode {
        TipeJit::Angka => NativeFn::Angka(unsafe { std::mem::transmute::<*const u8, extern "C" fn(*const i64, *mut i64) -> i64>(ptr) }),
        TipeJit::Desimal => NativeFn::Desimal(unsafe { std::mem::transmute::<*const u8, extern "C" fn(*const f64) -> f64>(ptr) }),
        TipeJit::Campur => unreachable!("dicegah di atas"),
    })
}

fn jalankan_stmt_list(program: Vec<(usize, Stmt)>) -> Result<(), String> {
    let mut resolver = Resolver::new();
    let top_level = resolver.resolve_top(&program).map_err(|e| format!("Kesalahan Kompilasi: {}", e))?;
    let top_level = optimisasi_blok(top_level);
    for cf in resolver.fungsi_out.values_mut() {
        if let Some(cf) = Rc::get_mut(cf) {
            cf.body = optimisasi_blok(std::mem::take(&mut cf.body));
        }
    }

    let mut nama_fungsi: Vec<String> = resolver.fungsi_out.keys().cloned().collect();
    nama_fungsi.sort(); // deterministik: HashMap tidak menjamin urutan iterasi konsisten antar-run,
    // jadi kalau tidak diurutkan, urutan fungsi (dan index-nya) di bundel .isoweb.json bisa
    // berbeda-beda tiap kali di-compile ulang walau source-nya sama persis (nondeterminism
    // bawaan, ditemukan sewaktu verifikasi representasi Daftar flat -- lihat ROADMAP.md).
    let fungsi_index: HashMap<String, usize> = nama_fungsi.iter().enumerate().map(|(i, n)| (n.clone(), i)).collect();

    let mut compiler = Compiler::new(fungsi_index);
    let top_kode = compiler.compile_top(&top_level);
    #[cfg(feature = "jit")]
    let mut jit = JitEngine::new();
    // ISOTERI_NO_JIT=1 matikan JIT sama sekali (semua fungsi lari ke bytecode VM murni),
    // TANPA perlu edit/hapus anotasi tipe di kode sumbernya. Dipakai scripts/regresi.sh
    // supaya bisa bandingkan hasil "bytecode murni" vs "JIT produksi" vs "via-ir" buat
    // 3 jalur eksekusi yang independen -- persis metodologi yang nemuin bug wrap-around
    // overflow JIT (lihat KETERBATASAN.md & docs/IR.md): kalau bytecode & JIT kasih hasil
    // beda buat program yang SAMA, itu tandanya salah satu jalurnya (biasanya JIT-nya,
    // karena lebih jarang dites manual) punya bug tersembunyi. Tanpa fitur "jit" (mis.
    // isoteri-wasm/) paksa_bytecode SELALU true -- gak ada JitEngine sama sekali.
    let paksa_bytecode = cfg!(not(feature = "jit")) || std::env::var("ISOTERI_NO_JIT").map(|v| v == "1").unwrap_or(false);
    let mut fungsi_vm: Vec<Rc<VMFungsi>> = Vec::with_capacity(nama_fungsi.len());
    for nama in &nama_fungsi {
        let cf = resolver.fungsi_out.get(nama).unwrap();
        #[cfg_attr(not(feature = "jit"), allow(unused_mut))]
        let mut vmf = compiler.compile_fungsi(cf);
        #[cfg_attr(not(feature = "jit"), allow(unused_variables))]
        if let Some(mode) = cf.tipe_jit {
            if paksa_bytecode {
                // sengaja skip -- vmf.native tetap None, VM otomatis pakai bytecode biasa.
            } else {
                #[cfg(feature = "jit")]
                match coba_kompilasi_jit(&mut jit, cf, mode) {
                    Ok(native) => vmf.native = Some(native),
                    Err(e) => eprintln!("Peringatan: fungsi \"{}\" gagal dikompilasi JIT ({}), pakai bytecode biasa.", nama, e),
                }
            }
        }
        fungsi_vm.push(Rc::new(vmf));
    }

    let mut vm = VM::new(resolver.global_count, compiler.konstanta, fungsi_vm, compiler.fungsi_index);
    vm.jalankan_top(&top_kode).map_err(|e| format!("Kesalahan Runtime: {}", e))
}

// =====================================================================
// 8b. ISOTERI IR LINEAR (tiga-alamat, typed) -- lanjutan docs/IR.md poin 2
// =====================================================================
//
// IR v1 (bagian 4b) masih berbentuk POHON (CExpr rekursif) -- cukup buat constant
// folding & dead code elimination, tapi BELUM cukup buat SIMD/vectorization yang
// benar (butuh urutan operasi yang eksplisit & linear, bukan pohon) atau buat
// menyatukan backend JIT dengan backend bytecode (keduanya masih menelusuri
// CExpr/CStmt sendiri-sendiri secara terpisah).
//
// IR LINEAR ini adalah lapisan BARU di antara IR v1 dan backend: representasi
// tiga-alamat (typed, register virtual) yang di-lower dari CStmt/CExpr, LALU
// di-lower LAGI jadi Instr (bytecode) supaya bisa langsung dijalankan & diuji
// terhadap jalur lama -- lihat `isoteri via-ir program.iso` di main.rs dan
// `runtime/web/README.md` gaya validasinya (bandingkan dua jalur independen,
// harus identik).
//
// PENTING: jalur produksi (jalankan_stmt_list, ekspor_json_dari_sumber) BELUM
// dipindah ke IR linear ini -- ini FONDASI yang sudah diverifikasi benar, siap
// dipakai backend JIT/SIMD/AOT berikutnya, tapi migrasi backend produksi itu
// sendiri didokumentasikan sebagai kerja lanjutan (docs/IR.md poin 3).
//
// Register 0..local_slot_count SAMA PERSIS dengan slot lokal yang sudah ada
// (CExpr::Local(n) langsung jadi register n, TANPA instruksi tambahan) --
// register di atas itu adalah TEMPORARY BARU yang dialokasikan selama lowering
// buat menyimpan hasil sub-ekspresi (di jalur stack lama ini implisit ada di
// stack; di sini eksplisit, itulah maksud "linear/typed").
//
// Tipe (IrType) diturunkan secara bottom-up dari literal & slot_tipe yang SUDAH
// dihitung Resolver buat elig-JIT (bagian "3. RESOLVER") -- bukan type-checker
// baru dari nol. Simpul yang tipenya belum bisa dipastikan statis (panggilan
// fungsi, daftar, peta, field, dst) jatuh ke IrType::Dinamis, yang berarti
// "berlaku seperti Value biasa sekarang" -- aman, cuma belum optimal.
//
// Simpul yang BELUM "dilinearkan" murni (SimpanLaluField/UbahField* dgn path
// nested, dan 'ulang selaras' yang badannya AST mentah) SENGAJA tidak dipaksa
// masuk representasi register -- dipakaikan escape hatch `IrInstr::Legacy`
// yang membungkus potongan Instr hasil Compiler lama apa adanya. ini konsisten
// dengan Hukum 6 "Explicit escape hatch" di docs/FILOSOFI.md: jangan buru-buru
// menggeneralisasi kasus langka kalau itu menambah risiko buat manfaat kecil.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IrType { Angka, Desimal, Teks, Bool, Dinamis }

#[derive(Debug, Clone)]
enum IrConst { Angka(i64), Desimal(f64), Teks(String), Bool(bool) }

type Reg = u32;

#[derive(Debug, Clone)]
enum IrInstr {
    Const(Reg, IrConst),
    LoadGlobal(Reg, usize),
    StoreGlobal(usize, Reg),
    Move(Reg, Reg),
    BinOp(Reg, BinOp, Reg, Reg),
    Tidak(Reg, Reg),
    MakeDaftar(Reg, Vec<Reg>),
    MakePeta(Reg, Vec<Rc<str>>, Vec<Reg>),
    Indeks(Reg, Reg, Reg),
    AmbilField(Reg, Reg, String),
    BuatInstans(Reg, Rc<str>, Vec<Rc<str>>, Vec<Reg>),
    BuatFungsi(Reg, usize, Vec<Reg>),
    PanggilFungsi(Reg, usize, Vec<Reg>),
    PanggilBawaan(Reg, String, Vec<Reg>),
    PanggilNilai(Reg, Reg, Vec<Reg>),
    Tampilkan(Reg),
    Jump(usize),
    JumpJikaSalah(Reg, usize),
    IterMulai(Reg),
    /// dst = item berikutnya; kalau iterator sudah habis, lompat ke `usize` (di-backpatch).
    IterLanjut(Reg, usize),
    /// target tangkap (di-backpatch), register tempat pesan galat ditaruh kalau kena.
    MulaiCoba(usize, Reg),
    SelesaiCoba,
    Kembalikan(Reg),
    TandaiBaris(usize),
    /// Escape hatch (lihat catatan di atas modul) -- Instr apa adanya, TANPA lompatan
    /// internal (baik nested-field-path maupun JalankanSelaras sama-sama straight-line
    /// dari sisi IR ini), jadi aman disisipkan di posisi berapa pun tanpa rebase offset.
    Legacy(Vec<Instr>, Option<Reg>),
}

/// Konteks satu loop yang lagi dilower ke IR -- sama persis semantiknya dengan
/// Compiler::LoopCtx (bytecode), lihat catatan di sana. Dipisah jadi struct sendiri
/// (bukan reuse LoopCtx langsung) karena field `break_patches`/`continue_target` di sini
/// menunjuk indeks di larik `IrInstr` (Vec<IrInstr>), BUKAN larik `Instr` (bytecode) --
/// dua ruang indeks yang berbeda.
struct LoopCtxIr {
    continue_target: usize,
    break_patches: Vec<usize>,
    coba_depth_saat_masuk: usize,
}

struct IrLower<'a> {
    kompiler: &'a mut Compiler, // dipakai ulang buat konstanta & fungsi_index & escape hatch
    reg_types: Vec<IrType>,     // terindeks per register; tumbuh seiring temp baru dialokasi
    slot_tipe: &'a [Option<TipeJit>],
    /// 'putus'/'lanjut' -- lihat catatan panjang di CStmt::Putus/Lanjut di bawah.
    loop_stack: Vec<LoopCtxIr>,
    /// SENGAJA field terpisah dari Compiler::coba_depth (walau namanya sama) -- coba_depth
    /// milik `self.kompiler` itu buat compile_stmt/compile_expr escape hatch punya Compiler
    /// SENDIRI (dipanggil buat statement lain semacam UbahFieldLocal, TIDAK ADA hubungannya
    /// dengan CobaLocal/CobaGlobal yang di sini dilower LANGSUNG ke IrInstr::MulaiCoba/
    /// SelesaiCoba, bukan lewat compile_stmt). Jadi coba_depth counter buat 'putus'/'lanjut'
    /// perlu dihitung ulang di sini, independen, dengan pola PERSIS sama seperti Compiler.
    coba_depth: usize,
}

impl<'a> IrLower<'a> {
    fn baru_reg(&mut self, t: IrType) -> Reg {
        self.reg_types.push(t);
        (self.reg_types.len() - 1) as u32
    }

    fn tipe_dari_jit(t: Option<TipeJit>) -> IrType {
        match t {
            Some(TipeJit::Angka) => IrType::Angka,
            Some(TipeJit::Desimal) => IrType::Desimal,
            None => IrType::Dinamis,
            // TipeJit::Campur itu status level FUNGSI (campuran antar-slot), bukan nilai yang
            // pernah tersimpan di SATU slot_tipe[i] individual -- tiap slot tetap Angka/Desimal/
            // None seperti biasa, cuma KOMBINASINYA yang bisa campur. Lihat enum TipeJit.
            Some(TipeJit::Campur) => unreachable!("slot_tipe per-elemen tidak seharusnya pernah Campur"),
        }
    }

    /// Lower satu ekspresi -> (register hasil, tipenya). Rekursif, bottom-up.
    /// Alokasi register tujuan buat satu simpul: kalau `dest` diberikan (destination-passing,
    /// dipakai `ingat`/`ubah` supaya hasil ekspresi ditulis LANGSUNG ke slot tujuan tanpa lewat
    /// register perantara + Move), pakai itu; kalau tidak, alokasikan temp baru seperti biasa.
    fn reg_tujuan(&mut self, dest: Option<Reg>, t: IrType) -> Reg {
        match dest {
            Some(r) => {
                while self.reg_types.len() <= r as usize { self.reg_types.push(IrType::Dinamis); }
                self.reg_types[r as usize] = t;
                r
            }
            None => self.baru_reg(t),
        }
    }

    fn lower_expr(&mut self, e: &CExpr, out: &mut Vec<IrInstr>) -> (Reg, IrType) {
        self.lower_expr_ke(e, out, None)
    }

    /// Sama seperti `lower_expr`, tapi kalau `dest` diberikan, hasil ekspresi ditulis LANGSUNG
    /// ke register itu -- inti dari optimasi register allocation v1 (lihat docs/IR.md):
    /// menghilangkan pola `BinOp(temp, ...); Move(slot, temp)` yang tadinya muncul di SETIAP
    /// `ubah x = ...`/`ingat x = ...`, jadi cukup `BinOp(slot, ...)` langsung.
    fn lower_expr_ke(&mut self, e: &CExpr, out: &mut Vec<IrInstr>, dest: Option<Reg>) -> (Reg, IrType) {
        match e {
            CExpr::Angka(n) => { let r = self.reg_tujuan(dest, IrType::Angka); out.push(IrInstr::Const(r, IrConst::Angka(*n))); (r, IrType::Angka) }
            CExpr::Desimal(f) => { let r = self.reg_tujuan(dest, IrType::Desimal); out.push(IrInstr::Const(r, IrConst::Desimal(*f))); (r, IrType::Desimal) }
            CExpr::Teks(s) => { let r = self.reg_tujuan(dest, IrType::Teks); out.push(IrInstr::Const(r, IrConst::Teks(s.clone()))); (r, IrType::Teks) }
            CExpr::Bool(b) => { let r = self.reg_tujuan(dest, IrType::Bool); out.push(IrInstr::Const(r, IrConst::Bool(*b))); (r, IrType::Bool) }
            CExpr::Global(slot) => { let r = self.reg_tujuan(dest, IrType::Dinamis); out.push(IrInstr::LoadGlobal(r, *slot)); (r, IrType::Dinamis) }
            CExpr::Local(slot) => {
                let src = *slot as u32;
                let t = self.reg_types.get(src as usize).copied().unwrap_or(IrType::Dinamis);
                match dest {
                    // `x = y` (beda register) -- satu-satunya kasus yang TETAP butuh Move,
                    // karena nilainya memang harus benar-benar dipindah, bukan cuma "dihasilkan
                    // di tempat lain". Kalau dest == src (mis. `x = x`), tidak perlu apa-apa.
                    Some(d) if d != src => { out.push(IrInstr::Move(d, src)); self.reg_types[d as usize] = t; (d, t) }
                    _ => (src, t),
                }
            }
            CExpr::Binary(l, op, r) => {
                let (lr, lt) = self.lower_expr(l, out);
                let (rr, rt) = self.lower_expr(r, out);
                let tipe_hasil = tipe_hasil_binop(*op, lt, rt);
                let dst = self.reg_tujuan(dest, tipe_hasil);
                out.push(IrInstr::BinOp(dst, *op, lr, rr));
                (dst, tipe_hasil)
            }
            CExpr::Tidak(e) => {
                let (er, _) = self.lower_expr(e, out); // truthy() diterapkan saat eksekusi (Instr::Tidak) -- tipe operand apa aja sah
                let dst = self.reg_tujuan(dest, IrType::Bool);
                out.push(IrInstr::Tidak(dst, er));
                (dst, IrType::Bool)
            }
            CExpr::Panggil(nama, args) => {
                let arg_regs: Vec<Reg> = args.iter().map(|a| self.lower_expr(a, out).0).collect();
                let dst = self.reg_tujuan(dest, IrType::Dinamis);
                if let Some(&idx) = self.kompiler.fungsi_index.get(nama) {
                    out.push(IrInstr::PanggilFungsi(dst, idx, arg_regs));
                } else {
                    out.push(IrInstr::PanggilBawaan(dst, nama.clone(), arg_regs));
                }
                (dst, IrType::Dinamis)
            }
            CExpr::Daftar(items) => {
                let regs: Vec<Reg> = items.iter().map(|i| self.lower_expr(i, out).0).collect();
                let dst = self.reg_tujuan(dest, IrType::Dinamis);
                out.push(IrInstr::MakeDaftar(dst, regs));
                (dst, IrType::Dinamis)
            }
            CExpr::Peta(entries) => {
                let kunci: Vec<Rc<str>> = entries.iter().map(|(k, _)| Rc::from(k.as_str())).collect();
                let regs: Vec<Reg> = entries.iter().map(|(_, v)| self.lower_expr(v, out).0).collect();
                let dst = self.reg_tujuan(dest, IrType::Dinamis);
                out.push(IrInstr::MakePeta(dst, kunci, regs));
                (dst, IrType::Dinamis)
            }
            CExpr::Indeks(t, i) => {
                let (tr, _) = self.lower_expr(t, out);
                let (ir, _) = self.lower_expr(i, out);
                let dst = self.reg_tujuan(dest, IrType::Dinamis);
                out.push(IrInstr::Indeks(dst, tr, ir));
                (dst, IrType::Dinamis)
            }
            CExpr::Field(t, f) => {
                let (tr, _) = self.lower_expr(t, out);
                let dst = self.reg_tujuan(dest, IrType::Dinamis);
                out.push(IrInstr::AmbilField(dst, tr, f.clone()));
                (dst, IrType::Dinamis)
            }
            CExpr::BentukLiteral(nama, entries) => {
                let field_nama: Vec<Rc<str>> = entries.iter().map(|(k, _)| Rc::from(k.as_str())).collect();
                let regs: Vec<Reg> = entries.iter().map(|(_, v)| self.lower_expr(v, out).0).collect();
                let dst = self.reg_tujuan(dest, IrType::Dinamis);
                out.push(IrInstr::BuatInstans(dst, Rc::from(nama.as_str()), field_nama, regs));
                (dst, IrType::Dinamis)
            }
            CExpr::FungsiLiteral(nama_sintetis, tangkapan) => {
                let regs: Vec<Reg> = tangkapan.iter().map(|e| self.lower_expr(e, out).0).collect();
                let idx = *self.kompiler.fungsi_index.get(nama_sintetis)
                    .unwrap_or_else(|| panic!("Closure \"{}\" tidak terdaftar -- bug internal resolver.", nama_sintetis));
                let dst = self.reg_tujuan(dest, IrType::Dinamis);
                out.push(IrInstr::BuatFungsi(dst, idx, regs));
                (dst, IrType::Dinamis)
            }
            CExpr::PanggilNilai(callee, args) => {
                let (fr, _) = self.lower_expr(callee, out);
                let arg_regs: Vec<Reg> = args.iter().map(|a| self.lower_expr(a, out).0).collect();
                let dst = self.reg_tujuan(dest, IrType::Dinamis);
                out.push(IrInstr::PanggilNilai(dst, fr, arg_regs));
                (dst, IrType::Dinamis)
            }
            // Escape hatch: nested field-path punya urutan Dup/AmbilField yang cukup rumit buat
            // diregisterkan murni dengan manfaat kecil (kasusnya jarang) -- pakai ulang Compiler
            // yang sudah teruji, dibungkus utuh sebagai satu blok Legacy straight-line.
            CExpr::SimpanLaluField(_, _, _) => {
                let mut kode = Vec::new();
                self.kompiler.compile_expr(e, &mut kode);
                let dst = self.reg_tujuan(dest, IrType::Dinamis);
                out.push(IrInstr::Legacy(kode, Some(dst)));
                (dst, IrType::Dinamis)
            }
        }
    }

    fn lower_blok(&mut self, stmts: &[(usize, CStmt)], out: &mut Vec<IrInstr>) {
        for (baris, s) in stmts {
            out.push(IrInstr::TandaiBaris(*baris));
            self.lower_stmt(s, out);
        }
    }

    fn lower_stmt(&mut self, s: &CStmt, out: &mut Vec<IrInstr>) {
        match s {
            CStmt::IngatGlobal(slot, e) => {
                let (r, _) = self.lower_expr(e, out);
                out.push(IrInstr::StoreGlobal(*slot, r));
            }
            CStmt::UbahGlobal(slot, e) => {
                // Peephole SAMA seperti Compiler::compile_stmt (lihat catatan panjang di
                // ekstrak_item_gabung_diri()/tambahkan_elemen_inplace()) -- dibungkus lewat
                // escape hatch IrInstr::Legacy karena Instr::TambahkanGlobal langsung menulis
                // ke slot (tidak butuh register tujuan/StoreGlobal terpisah seperti jalur umum).
                if let Some(item) = ekstrak_item_gabung_diri(e, SlotSasaran::Global(*slot)) {
                    let (r, _) = self.lower_expr(item, out);
                    out.push(IrInstr::Legacy(vec![Instr::LoadLocal(r as usize), Instr::TambahkanGlobal(*slot)], None));
                } else {
                    let (r, _) = self.lower_expr(e, out);
                    out.push(IrInstr::StoreGlobal(*slot, r));
                }
            }
            CStmt::IngatLocal(slot, e) => {
                self.lower_expr_ke(e, out, Some(*slot as u32));
            }
            CStmt::UbahLocal(slot, e) => {
                if let Some(item) = ekstrak_item_gabung_diri(e, SlotSasaran::Lokal(*slot)) {
                    let (r, _) = self.lower_expr(item, out);
                    out.push(IrInstr::Legacy(vec![Instr::LoadLocal(r as usize), Instr::TambahkanLokal(*slot)], None));
                } else {
                    self.lower_expr_ke(e, out, Some(*slot as u32));
                }
            }
            CStmt::UbahFieldGlobal(_, _, _) | CStmt::UbahFieldLocal(_, _, _)
            | CStmt::UbahJalurGlobal(_, _, _) | CStmt::UbahJalurLocal(_, _, _) => {
                // Escape hatch yang sama seperti SimpanLaluField -- lihat catatan di atas.
                // UbahJalur (assignment via indeks) AMAN lewat sini karena compile_set_jalur
                // cuma menghasilkan instruksi lurus (Dup/Indeks/IndeksTahanIdx/SetField/
                // SetIndeks) TANPA lompatan internal apa pun -- persis prasyarat Legacy.
                let mut kode = Vec::new();
                self.kompiler.compile_stmt(s, &mut kode);
                out.push(IrInstr::Legacy(kode, None));
            }
            CStmt::Tampilkan(e) => { let (r, _) = self.lower_expr(e, out); out.push(IrInstr::Tampilkan(r)); }
            CStmt::Kalau(cond, tb, eb) => {
                let (cr, _) = self.lower_expr(cond, out);
                let lompat_salah_idx = out.len();
                out.push(IrInstr::JumpJikaSalah(cr, 0));
                self.lower_blok(tb, out);
                if let Some(eb) = eb {
                    let lompat_akhir_idx = out.len();
                    out.push(IrInstr::Jump(0));
                    let else_mulai = out.len();
                    if let IrInstr::JumpJikaSalah(_, t) = &mut out[lompat_salah_idx] { *t = else_mulai; }
                    self.lower_blok(eb, out);
                    let akhir = out.len();
                    if let IrInstr::Jump(t) = &mut out[lompat_akhir_idx] { *t = akhir; }
                } else {
                    let akhir = out.len();
                    if let IrInstr::JumpJikaSalah(_, t) = &mut out[lompat_salah_idx] { *t = akhir; }
                }
            }
            CStmt::Ulang(cond, body) => {
                let mulai = out.len();
                let (cr, _) = self.lower_expr(cond, out);
                let lompat_salah_idx = out.len();
                out.push(IrInstr::JumpJikaSalah(cr, 0));
                self.loop_stack.push(LoopCtxIr { continue_target: mulai, break_patches: Vec::new(), coba_depth_saat_masuk: self.coba_depth });
                self.lower_blok(body, out);
                let ctx = self.loop_stack.pop().unwrap();
                out.push(IrInstr::Jump(mulai));
                let akhir = out.len();
                if let IrInstr::JumpJikaSalah(_, t) = &mut out[lompat_salah_idx] { *t = akhir; }
                for idx in ctx.break_patches { if let IrInstr::Jump(t) = &mut out[idx] { *t = akhir; } }
            }
            CStmt::UlangSetiapGlobal(slot, e, body) => {
                let (er, _) = self.lower_expr(e, out);
                out.push(IrInstr::IterMulai(er));
                let mulai = out.len();
                let dst = self.baru_reg(IrType::Dinamis);
                out.push(IrInstr::IterLanjut(dst, 0));
                out.push(IrInstr::StoreGlobal(*slot, dst));
                self.loop_stack.push(LoopCtxIr { continue_target: mulai, break_patches: Vec::new(), coba_depth_saat_masuk: self.coba_depth });
                self.lower_blok(body, out);
                let ctx = self.loop_stack.pop().unwrap();
                out.push(IrInstr::Jump(mulai));
                let akhir = out.len();
                if let IrInstr::IterLanjut(_, t) = &mut out[mulai] { *t = akhir; }
                for idx in ctx.break_patches { if let IrInstr::Jump(t) = &mut out[idx] { *t = akhir; } }
            }
            CStmt::UlangSetiapLocal(slot, e, body) => {
                let (er, _) = self.lower_expr(e, out);
                out.push(IrInstr::IterMulai(er));
                let mulai = out.len();
                while self.reg_types.len() <= *slot { self.reg_types.push(IrType::Dinamis); }
                self.reg_types[*slot] = IrType::Dinamis;
                out.push(IrInstr::IterLanjut(*slot as u32, 0));
                self.loop_stack.push(LoopCtxIr { continue_target: mulai, break_patches: Vec::new(), coba_depth_saat_masuk: self.coba_depth });
                self.lower_blok(body, out);
                let ctx = self.loop_stack.pop().unwrap();
                out.push(IrInstr::Jump(mulai));
                let akhir = out.len();
                if let IrInstr::IterLanjut(_, t) = &mut out[mulai] { *t = akhir; }
                for idx in ctx.break_patches { if let IrInstr::Jump(t) = &mut out[idx] { *t = akhir; } }
            }
            CStmt::UlangSelaras(e, _, _) => {
                // Badannya AST Stmt mentah (bukan bytecode) -- lihat catatan modul. Pakai ulang
                // Compiler apa adanya (menghasilkan satu Instr::JalankanSelaras, straight-line).
                let mut kode = Vec::new();
                self.kompiler.compile_stmt(s, &mut kode);
                let _ = e; // sudah ikut ke dalam `kode` lewat compile_stmt
                out.push(IrInstr::Legacy(kode, None));
            }
            CStmt::CobaGlobal(badan_coba, slot, badan_tangkap) => {
                let mulai_idx = out.len();
                let dst_pesan = self.baru_reg(IrType::Teks);
                out.push(IrInstr::MulaiCoba(0, dst_pesan));
                self.coba_depth += 1;
                self.lower_blok(badan_coba, out);
                self.coba_depth -= 1;
                out.push(IrInstr::SelesaiCoba);
                let lompat_akhir_idx = out.len();
                out.push(IrInstr::Jump(0));
                let target_tangkap = out.len();
                if let IrInstr::MulaiCoba(t, _) = &mut out[mulai_idx] { *t = target_tangkap; }
                out.push(IrInstr::StoreGlobal(*slot, dst_pesan));
                self.lower_blok(badan_tangkap, out);
                let akhir = out.len();
                if let IrInstr::Jump(t) = &mut out[lompat_akhir_idx] { *t = akhir; }
            }
            CStmt::CobaLocal(badan_coba, slot, badan_tangkap) => {
                let mulai_idx = out.len();
                // Tulis pesan galat LANGSUNG ke slot tangkap, tidak perlu temp+Move terpisah
                // (beda dengan sebelumnya) -- registernya sendiri "ditumpangi" MulaiCoba/
                // penangkap error VM, jadi ini aman: sebelum SelesaiCoba tak ada yang baca
                // slot itu (badan_coba belum tentu menulisinya buat hal lain).
                let dst_pesan = self.reg_tujuan(Some(*slot as u32), IrType::Teks);
                out.push(IrInstr::MulaiCoba(0, dst_pesan));
                self.coba_depth += 1;
                self.lower_blok(badan_coba, out);
                self.coba_depth -= 1;
                out.push(IrInstr::SelesaiCoba);
                let lompat_akhir_idx = out.len();
                out.push(IrInstr::Jump(0));
                let target_tangkap = out.len();
                if let IrInstr::MulaiCoba(t, _) = &mut out[mulai_idx] { *t = target_tangkap; }
                self.lower_blok(badan_tangkap, out);
                let akhir = out.len();
                if let IrInstr::Jump(t) = &mut out[lompat_akhir_idx] { *t = akhir; }
            }
            CStmt::Kembalikan(e) => { let (r, _) = self.lower_expr(e, out); out.push(IrInstr::Kembalikan(r)); }
            CStmt::EkspresiStmt(e) => { self.lower_expr(e, out); }
            CStmt::Putus => {
                // Sama persis semantiknya dengan Compiler::compile_stmt CStmt::Putus (bytecode)
                // -- lihat catatan panjang di LoopCtx/LoopCtxIr. Kalau 'putus' melompat keluar
                // dari tengah satu atau lebih blok 'coba' aktif (coba_depth sekarang lebih besar
                // dari coba_depth SAAT loop yang dituju baru mulai), handler_stack VM butuh
                // di-"tutup" sebanyak selisihnya SEBELUM lompat -- kalau tidak, handler yang
                // seharusnya sudah tidak aktif tetap nyangkut aktif buat kode setelah loop.
                // Instr::TutupHandler sudah ada & teruji lewat jalur bytecode biasa, jadi
                // dipakai ulang APA ADANYA lewat escape hatch Legacy (parameterless, tidak
                // nyentuh register, aman disisipkan di posisi manapun).
                let ctx = self.loop_stack.last().expect("resolver sudah memvalidasi 'putus' cuma ada di dalam loop");
                let n_tutup = self.coba_depth - ctx.coba_depth_saat_masuk;
                if n_tutup > 0 { out.push(IrInstr::Legacy(vec![Instr::TutupHandler; n_tutup], None)); }
                let idx = out.len();
                out.push(IrInstr::Jump(0));
                self.loop_stack.last_mut().unwrap().break_patches.push(idx);
            }
            CStmt::Lanjut => {
                let ctx = self.loop_stack.last().expect("resolver sudah memvalidasi 'lanjut' cuma ada di dalam loop");
                let n_tutup = self.coba_depth - ctx.coba_depth_saat_masuk;
                let target = ctx.continue_target;
                if n_tutup > 0 { out.push(IrInstr::Legacy(vec![Instr::TutupHandler; n_tutup], None)); }
                out.push(IrInstr::Jump(target));
            }
        }
    }
}

/// Tipe hasil BinOp secara statis, kalau bisa dipastikan dari tipe kedua operand -- dipakai
/// murni buat metadata IR (register-typing), belum mempengaruhi bytecode yang dihasilkan
/// backend IR->Instr (itu tetap dinamis/Value seperti sekarang, lihat catatan di atas modul).
fn tipe_hasil_binop(op: BinOp, l: IrType, r: IrType) -> IrType {
    use BinOp::*;
    match op {
        SamaDengan | TidakSama | LebihBesar | LebihBesarSama | LebihKecil | LebihKecilSama | Dan | Atau => {
            if l != IrType::Dinamis && r != IrType::Dinamis { IrType::Bool } else { IrType::Dinamis }
        }
        Tambah if l == IrType::Teks || r == IrType::Teks => IrType::Teks,
        Tambah | Kurang | Kali | Bagi | Modulo => match (l, r) {
            (IrType::Angka, IrType::Angka) => IrType::Angka,
            (IrType::Angka, IrType::Desimal) | (IrType::Desimal, IrType::Angka) | (IrType::Desimal, IrType::Desimal) => IrType::Desimal,
            _ => IrType::Dinamis,
        },
    }
}

/// Lower satu fungsi (body CFungsi) -> (instruksi IR, tipe register, jumlah register total).
fn lower_fungsi_ke_ir(kompiler: &mut Compiler, cf: &CFungsi) -> (Vec<IrInstr>, Vec<IrType>) {
    let mut reg_types: Vec<IrType> = (0..cf.local_slot_count)
        .map(|i| IrLower::tipe_dari_jit(cf.slot_tipe.get(i).copied().flatten()))
        .collect();
    let mut out = Vec::new();
    {
        let mut lower = IrLower { kompiler, reg_types: std::mem::take(&mut reg_types), slot_tipe: &cf.slot_tipe, loop_stack: Vec::new(), coba_depth: 0 };
        lower.lower_blok(&cf.body, &mut out);
        reg_types = lower.reg_types;
    }
    (out, reg_types)
}

fn lower_top_ke_ir(kompiler: &mut Compiler, top: &[(usize, CStmt)]) -> (Vec<IrInstr>, Vec<IrType>) {
    let mut out = Vec::new();
    let reg_types;
    {
        let mut lower = IrLower { kompiler, reg_types: Vec::new(), slot_tipe: &[], loop_stack: Vec::new(), coba_depth: 0 };
        lower.lower_blok(top, &mut out);
        reg_types = lower.reg_types;
    }
    (out, reg_types)
}

/// Backend IR-linear -> Instr (stack bytecode): tiap register jadi "slot lokal" tambahan
/// (lihat `local_slot_count` yang dikembalikan -- lebih besar dari punya CFungsi asli karena
/// menampung register temporary juga). Tipe (IrType) SENGAJA diabaikan di sini -- bytecode VM
/// sudah dinamis (Value bertag) dari sononya, jadi tidak ada untungnya membedakan representasi
/// per tipe di jalur ini. Nilai tipe itu baru kepakai kalau IR ini suatu saat diberi backend
/// JIT/SIMD sendiri (lihat docs/IR.md poin 2-3) yang BISA memakai register unboxed native.
/// Lower IR linear -> Instr (stack bytecode): tiap register jadi "slot lokal" tambahan
/// (lihat `local_slot_count` yang dikembalikan pemanggil -- lebih besar dari punya CFungsi
/// asli karena menampung register temporary juga). Tipe (IrType) SENGAJA diabaikan di sini --
/// bytecode VM sudah dinamis (Value bertag) dari sononya, jadi tidak ada untungnya membedakan
/// representasi per tipe di jalur ini. Nilai tipe itu baru kepakai kalau IR ini suatu saat
/// diberi backend JIT/SIMD sendiri (lihat docs/IR.md poin 2-3) yang BISA memakai register
/// unboxed native. Butuh akses ke `Compiler::tambah_konstanta` buat instruksi `Const`.
fn ir_ke_instr_dgn_konstanta(kompiler: &mut Compiler, ir: &[IrInstr]) -> Vec<Instr> {
    // Pass 1: hitung index instruksi Instr AWAL setiap IrInstr (buat rebase target lompatan).
    let mut ukuran_per_instr: Vec<usize> = Vec::with_capacity(ir.len());
    for instr in ir {
        ukuran_per_instr.push(match instr {
            IrInstr::Const(..) => 2,          // PushK ; StoreLocal
            IrInstr::LoadGlobal(..) => 2,     // LoadGlobal ; StoreLocal
            IrInstr::StoreGlobal(..) => 2,    // LoadLocal ; StoreGlobal
            IrInstr::Move(..) => 2,           // LoadLocal ; StoreLocal
            IrInstr::BinOp(..) => 4,          // LoadLocal x2 ; BinOp ; StoreLocal
            IrInstr::Tidak(..) => 3,          // LoadLocal ; Tidak ; StoreLocal
            IrInstr::MakeDaftar(_, items) => items.len() + 2,
            IrInstr::MakePeta(_, _, v) => v.len() + 1 + 1,
            IrInstr::Indeks(..) => 4,
            IrInstr::AmbilField(..) => 3,
            IrInstr::BuatInstans(_, _, _, v) => v.len() + 1 + 1,
            IrInstr::BuatFungsi(_, _, t) => t.len() + 1 + 1,
            IrInstr::PanggilFungsi(_, _, a) => a.len() + 1 + 1,
            IrInstr::PanggilBawaan(_, _, a) => a.len() + 1 + 1,
            IrInstr::PanggilNilai(_, _, a) => a.len() + 2 + 1,
            IrInstr::Tampilkan(..) => 2,
            IrInstr::Jump(..) => 1,
            IrInstr::JumpJikaSalah(..) => 2,
            IrInstr::IterMulai(..) => 2,
            IrInstr::IterLanjut(..) => 1,
            IrInstr::MulaiCoba(..) => 1,
            IrInstr::SelesaiCoba => 1,
            IrInstr::Kembalikan(..) => 2,
            IrInstr::TandaiBaris(..) => 1,
            IrInstr::Legacy(k, dst) => k.len() + if dst.is_some() { 1 } else { 0 },
        });
    }
    let mut peta_awal = vec![0usize; ir.len() + 1];
    for i in 0..ir.len() { peta_awal[i + 1] = peta_awal[i] + ukuran_per_instr[i]; }

    let reb = |target: usize| -> usize { peta_awal.get(target).copied().unwrap_or_else(|| peta_awal[ir.len()]) };

    let mut out = Vec::with_capacity(peta_awal[ir.len()]);
    for instr in ir {
        match instr {
            IrInstr::Const(dst, c) => {
                let v = match c {
                    IrConst::Angka(n) => Value::Angka(*n),
                    IrConst::Desimal(f) => Value::Desimal(*f),
                    IrConst::Teks(s) => Value::Teks(s.clone().into()),
                    IrConst::Bool(b) => Value::Bool(*b),
                };
                let k = kompiler.tambah_konstanta(v);
                out.push(Instr::PushK(k));
                out.push(Instr::StoreLocal(*dst as usize));
            }
            IrInstr::LoadGlobal(dst, slot) => { out.push(Instr::LoadGlobal(*slot)); out.push(Instr::StoreLocal(*dst as usize)); }
            IrInstr::StoreGlobal(slot, src) => { out.push(Instr::LoadLocal(*src as usize)); out.push(Instr::StoreGlobal(*slot)); }
            IrInstr::Move(dst, src) => { out.push(Instr::LoadLocal(*src as usize)); out.push(Instr::StoreLocal(*dst as usize)); }
            IrInstr::BinOp(dst, op, a, b) => {
                out.push(Instr::LoadLocal(*a as usize));
                out.push(Instr::LoadLocal(*b as usize));
                out.push(Instr::BinOp(*op));
                out.push(Instr::StoreLocal(*dst as usize));
            }
            IrInstr::Tidak(dst, src) => {
                out.push(Instr::LoadLocal(*src as usize));
                out.push(Instr::Tidak);
                out.push(Instr::StoreLocal(*dst as usize));
            }
            IrInstr::MakeDaftar(dst, items) => {
                for r in items { out.push(Instr::LoadLocal(*r as usize)); }
                out.push(Instr::MakeDaftar(items.len()));
                out.push(Instr::StoreLocal(*dst as usize));
            }
            IrInstr::MakePeta(dst, kunci, nilai) => {
                for r in nilai { out.push(Instr::LoadLocal(*r as usize)); }
                out.push(Instr::MakePeta(kunci.clone()));
                out.push(Instr::StoreLocal(*dst as usize));
            }
            IrInstr::Indeks(dst, t, i) => {
                out.push(Instr::LoadLocal(*t as usize));
                out.push(Instr::LoadLocal(*i as usize));
                out.push(Instr::Indeks);
                out.push(Instr::StoreLocal(*dst as usize));
            }
            IrInstr::AmbilField(dst, t, f) => {
                out.push(Instr::LoadLocal(*t as usize));
                out.push(Instr::AmbilField(f.clone()));
                out.push(Instr::StoreLocal(*dst as usize));
            }
            IrInstr::BuatInstans(dst, nama, fields, regs) => {
                for r in regs { out.push(Instr::LoadLocal(*r as usize)); }
                out.push(Instr::BuatInstans(nama.clone(), fields.clone()));
                out.push(Instr::StoreLocal(*dst as usize));
            }
            IrInstr::BuatFungsi(dst, idx, tangkapan) => {
                for r in tangkapan { out.push(Instr::LoadLocal(*r as usize)); }
                out.push(Instr::BuatFungsi(*idx, tangkapan.len()));
                out.push(Instr::StoreLocal(*dst as usize));
            }
            IrInstr::PanggilFungsi(dst, idx, args) => {
                for r in args { out.push(Instr::LoadLocal(*r as usize)); }
                out.push(Instr::PanggilFungsi(*idx, args.len()));
                out.push(Instr::StoreLocal(*dst as usize));
            }
            IrInstr::PanggilBawaan(dst, nama, args) => {
                for r in args { out.push(Instr::LoadLocal(*r as usize)); }
                out.push(Instr::PanggilBawaan(nama.clone(), args.len()));
                out.push(Instr::StoreLocal(*dst as usize));
            }
            IrInstr::PanggilNilai(dst, f, args) => {
                out.push(Instr::LoadLocal(*f as usize));
                for r in args { out.push(Instr::LoadLocal(*r as usize)); }
                out.push(Instr::PanggilNilai(args.len()));
                out.push(Instr::StoreLocal(*dst as usize));
            }
            IrInstr::Tampilkan(r) => { out.push(Instr::LoadLocal(*r as usize)); out.push(Instr::Tampilkan); }
            IrInstr::Jump(t) => out.push(Instr::Lompat(reb(*t))),
            IrInstr::JumpJikaSalah(r, t) => { out.push(Instr::LoadLocal(*r as usize)); out.push(Instr::LompatJikaSalah(reb(*t))); }
            IrInstr::IterMulai(r) => { out.push(Instr::LoadLocal(*r as usize)); out.push(Instr::IterMulai); }
            IrInstr::IterLanjut(dst, t) => out.push(Instr::IterLanjutLocal(*dst as usize, reb(*t))),
            IrInstr::MulaiCoba(t, dst_pesan) => out.push(Instr::MulaiCobaLocal(reb(*t), *dst_pesan as usize)),
            IrInstr::SelesaiCoba => out.push(Instr::SelesaiCoba),
            IrInstr::Kembalikan(r) => { out.push(Instr::LoadLocal(*r as usize)); out.push(Instr::Kembalikan); }
            IrInstr::TandaiBaris(n) => out.push(Instr::TandaiBaris(*n)),
            IrInstr::Legacy(kode, dst) => {
                out.extend(kode.iter().cloned());
                if let Some(dst) = dst { out.push(Instr::StoreLocal(*dst as usize)); }
            }
        }
    }
    out
}

/// Jalur validasi (BUKAN jalur produksi -- lihat catatan di atas modul & docs/IR.md):
/// resolve+optimisasi IR-pohon seperti biasa, TAPI lower tiap fungsi & top-level lewat
/// IR LINEAR baru sebelum diubah balik jadi Instr, lalu jalankan seperti biasa. Dipanggil
/// dari `isoteri via-ir program.iso` di main.rs buat dibandingkan byte-per-byte terhadap
/// `isoteri program.iso` (jalur produksi/lama) -- lihat benchmarks/README kalau ada, atau
/// jalankan manual: diff <(isoteri p.iso) <(isoteri via-ir p.iso).
fn target_lompat(instr: &Instr) -> Option<usize> {
    match instr {
        Instr::Lompat(t) | Instr::LompatJikaSalah(t) => Some(*t),
        Instr::IterLanjutLocal(_, t) | Instr::IterLanjutGlobal(_, t) => Some(*t),
        Instr::MulaiCobaLocal(t, _) | Instr::MulaiCobaGlobal(t, _) => Some(*t),
        _ => None,
    }
}

fn rebase_target(instr: Instr, peta: &[usize]) -> Instr {
    match instr {
        Instr::Lompat(t) => Instr::Lompat(peta[t]),
        Instr::LompatJikaSalah(t) => Instr::LompatJikaSalah(peta[t]),
        Instr::IterLanjutLocal(s, t) => Instr::IterLanjutLocal(s, peta[t]),
        Instr::IterLanjutGlobal(s, t) => Instr::IterLanjutGlobal(s, peta[t]),
        Instr::MulaiCobaLocal(t, s) => Instr::MulaiCobaLocal(peta[t], s),
        Instr::MulaiCobaGlobal(t, s) => Instr::MulaiCobaGlobal(peta[t], s),
        lain => lain,
    }
}

/// Stack scheduling (lanjutan register allocation v1, lihat docs/IR.md) -- post-pass di atas
/// `Vec<Instr>` HASIL AKHIR (jump target sudah ter-resolve absolut), bukan di level IR linear
/// lagi, supaya tidak perlu mengurus reindexing IrInstr yang rumit.
///
/// PENTING -- riwayat perbaikan: versi pertama fungsi ini mengizinkan celah SEMBARANG antara
/// `StoreLocal(r)` dan `LoadLocal(r)` asal tidak ada instruksi kontrol alur di antaranya. Itu
/// TERBUKTI SALAH: kalau di celah ada instruksi lain yang men-*push* nilai (mis. `LoadLocal(0)`
/// buat operand LAIN dari BinOp yang sama), nilai `r` yang "dibiarkan nangkring" di stack jadi
/// ke-dahului nilai baru itu -- urutan operand kebalik (`n <= 1` diam-diam jadi `1 <= n`, lolos
/// dari validasi 17 program contoh untuk kasus non-rekursif tapi KETAHUAN lewat fungsi rekursif
/// `fib`/`faktorial` yang hasilnya jadi salah). Versi ini jauh lebih KONSERVATIF: cuma
/// menghapus pasangan yang BENAR-BENAR BERSEBELAHAN (`StoreLocal(r)` lalu PERSIS instruksi
/// berikutnya `LoadLocal(r)`, TANPA celah apa pun) -- itu selalu aman karena identik dengan
/// "simpan lalu langsung ambil lagi tanpa ada yang lain terjadi di antaranya", tidak ada ruang
/// buat instruksi lain menyerobot urutan stack. Cakupannya lebih kecil dari rencana awal, tapi
/// terbukti benar lewat regresi 17/17 (lihat docs/IR.md buat detail & rencana lanjutan yang
/// lebih general tapi tetap correct).
fn stack_scheduling(instrs: Vec<Instr>, ambang_temp: usize) -> Vec<Instr> {
    let n = instrs.len();
    let mut target_masuk: Vec<bool> = vec![false; n + 1];
    for ins in &instrs {
        if let Some(t) = target_lompat(ins) {
            if t <= n { target_masuk[t] = true; }
        }
    }

    let mut buang = vec![false; n];
    for i in 0..n.saturating_sub(1) {
        if let Instr::StoreLocal(r) = instrs[i] {
            if r >= ambang_temp && !target_masuk[i] && !target_masuk[i + 1] {
                if let Instr::LoadLocal(r2) = instrs[i + 1] {
                    if r2 == r { buang[i] = true; buang[i + 1] = true; }
                }
            }
        }
    }

    let mut peta_lama_baru = vec![0usize; n + 1];
    let mut baru_idx = 0;
    for k in 0..n {
        peta_lama_baru[k] = baru_idx;
        if !buang[k] { baru_idx += 1; }
    }
    peta_lama_baru[n] = baru_idx;

    instrs.into_iter().enumerate()
        .filter(|(k, _)| !buang[*k])
        .map(|(_, ins)| rebase_target(ins, &peta_lama_baru))
        .collect()
}

// Sama seperti coba_kompilasi_jit() tapi untuk jalur via-ir/AOT (kompilasi_dari_ir,
// bukan kompilasi biasa) -- lihat catatan di sana.
#[cfg(feature = "jit")]
fn coba_kompilasi_jit_dari_ir(jit: &mut JitEngine, nama: &str, ir: &[IrInstr], reg_types: &[IrType], param_count: usize, local_slot_count: usize, mode: TipeJit) -> Result<NativeFn, String> {
    let ptr = jit.kompilasi_dari_ir(nama, ir, reg_types, param_count, local_slot_count, mode)?;
    Ok(match mode {
        TipeJit::Angka => NativeFn::Angka(unsafe { std::mem::transmute::<*const u8, extern "C" fn(*const i64, *mut i64) -> i64>(ptr) }),
        TipeJit::Desimal => NativeFn::Desimal(unsafe { std::mem::transmute::<*const u8, extern "C" fn(*const f64) -> f64>(ptr) }),
        // Signature: satu pointer larik argumen (i64 mentah/bit-pattern f64 campur, lihat
        // catatan panjang di NativeFn::Campur), TANPA ptr flag overflow (Campur tidak pernah
        // aritmatika), kembalikan i64 (verified Angka-only lewat CStmt::Kembalikan check di
        // cek_jit_murni_stmt).
        TipeJit::Campur => NativeFn::Campur(unsafe { std::mem::transmute::<*const u8, extern "C" fn(*const i64) -> i64>(ptr) }),
    })
}

pub fn jalankan_stmt_list_via_ir(program: Vec<(usize, Stmt)>) -> Result<(), String> {
    let mut resolver = Resolver::new();
    let top_level = resolver.resolve_top(&program).map_err(|e| format!("Kesalahan Kompilasi: {}", e))?;
    let top_level = optimisasi_blok(top_level);
    for cf in resolver.fungsi_out.values_mut() {
        if let Some(cf) = Rc::get_mut(cf) {
            cf.body = optimisasi_blok(std::mem::take(&mut cf.body));
        }
    }

    let mut nama_fungsi: Vec<String> = resolver.fungsi_out.keys().cloned().collect();
    nama_fungsi.sort(); // deterministik: HashMap tidak menjamin urutan iterasi konsisten antar-run,
    // jadi kalau tidak diurutkan, urutan fungsi (dan index-nya) di bundel .isoweb.json bisa
    // berbeda-beda tiap kali di-compile ulang walau source-nya sama persis (nondeterminism
    // bawaan, ditemukan sewaktu verifikasi representasi Daftar flat -- lihat ROADMAP.md).
    let fungsi_index: HashMap<String, usize> = nama_fungsi.iter().enumerate().map(|(i, n)| (n.clone(), i)).collect();

    let mut compiler = Compiler::new(fungsi_index);

    let (top_ir, top_reg_types) = lower_top_ke_ir(&mut compiler, &top_level);
    let top_kode = ir_ke_instr_dgn_konstanta(&mut compiler, &top_ir);
    // Top-level tidak punya "local slot asli" sama sekali (semua lewat Global) -- jadi SEMUA
    // register (mulai 0) di sini adalah temporary, ambang_temp=0 aman & memaksimalkan peluang
    // stack scheduling (lihat docs/IR.md, ini justru target utama optimasi ini).
    let top_kode = stack_scheduling(top_kode, 0);

    let mut fungsi_vm: Vec<Rc<VMFungsi>> = Vec::with_capacity(nama_fungsi.len());
    #[cfg(feature = "jit")]
    let mut jit = JitEngine::new();
    for nama in &nama_fungsi {
        let cf = resolver.fungsi_out.get(nama).unwrap().clone();
        // param_flat (dukungan callback struct-flattened buat petakan/saring/urutkan) dipakai
        // ULANG dari compile_fungsi yang sudah teruji -- bukan bagian yang coba "dilinearkan"
        // di v1 ini (skopnya soal urutan operasi, bukan analisis parameter). Cuma .kode &
        // .local_slot_count dari hasil ini yang DIBUANG & diganti versi IR linear di bawah.
        let param_flat = compiler.compile_fungsi(&cf).param_flat;
        let (ir, reg_types) = lower_fungsi_ke_ir(&mut compiler, &cf);
        let kode = ir_ke_instr_dgn_konstanta(&mut compiler, &ir);
        let kode = stack_scheduling(kode, cf.local_slot_count);
        #[cfg_attr(not(feature = "jit"), allow(unused_mut))]
        let mut native = None;
        // Migrasi JIT (docs/IR.md poin 3): fungsi yang SAMA PERSIS lolos elig produksi
        // (cf.tipe_jit, dihitung Resolver -- tidak dihitung ulang di sini) SEKARANG JUGA
        // dicoba dikompilasi lewat IR linear yang baru, bukan cuma bytecode. Kalau gagal,
        // turun ke bytecode biasa (dari `kode` di atas) -- sama seperti perilaku produksi
        // saat JIT gagal, TIDAK fatal. Tanpa fitur "jit" (mis. isoteri-wasm/): `native`
        // otomatis tetap None, langsung lari ke bytecode `kode` -- SAMA seperti JIT gagal.
        #[cfg_attr(not(feature = "jit"), allow(unused_variables))]
        if let Some(mode) = cf.tipe_jit {
            #[cfg(feature = "jit")]
            match coba_kompilasi_jit_dari_ir(&mut jit, nama, &ir, &reg_types, cf.param_count, cf.local_slot_count, mode) {
                Ok(n) => native = Some(n),
                Err(e) => eprintln!("Peringatan (via-ir): fungsi \"{}\" gagal dikompilasi JIT-dari-IR ({}), pakai bytecode.", nama, e),
            }
        }
        let vmf = VMFungsi {
            param_count: cf.param_count,
            local_slot_count: reg_types.len().max(cf.local_slot_count),
            kode,
            native,
            param_flat,
            slot_tipe: cf.slot_tipe.clone(),
        };
        fungsi_vm.push(Rc::new(vmf));
    }

    let mut vm = VM::new(resolver.global_count, compiler.konstanta, fungsi_vm, compiler.fungsi_index);
    // Top-level (beda dari body fungsi) TIDAK pernah punya frame lokal yang dialokasikan
    // otomatis oleh PanggilFungsi (lihat eksekusi_satu) -- locals_stack top-level dimulai
    // kosong. IR linear butuh register 0..N buat temporary top-level, jadi pre-alokasikan
    // manual di sini SEBELUM jalan (aman: sama-sama modul ini, lihat definisi VM/VMState).
    vm.state.locals_stack.resize(top_reg_types.len(), Value::Kosong);
    vm.jalankan_top(&top_kode).map_err(|e| format!("Kesalahan Runtime (via-ir): {}", e))
}

pub fn jalankan_berkas_via_ir(path: &str) -> Result<(), String> {
    jalankan_stmt_list_via_ir(program_dari_berkas(path)?)
}

// =====================================================================
// 9. EKSPOR WEB: serialisasi bytecode ke JSON buat dijalankan lewat
//    runtime/web/isoteri-vm.js (VM tulis-ulang di JavaScript) -- Fase 3
//    blueprint ("Browser Native") tanpa perlu target wasm32-unknown-unknown
//    (yang butuh rustup, tidak tersedia di banyak environment CI/sandbox).
//
//    Strategi: bytecode Isoteri itu sendiri sudah representasi flat &
//    portable (Vec<Instr> berisi cuma angka/teks/enum sederhana) -- jadi
//    dump apa adanya ke JSON, lalu interpreter JS jalanin persis instruksi
//    yang sama alih-alih meng-compile ulang lewat Cranelift/wasm.
//
//    TIDAK diekspor: kode mesin hasil JIT (native fn pointer jelas gak
//    bisa diserialisasi -- browser tetap pakai jalur bytecode biasa utuh,
//    yang secara semantik identik, cuma lebih lambat) dan `ulang selaras`
//    (JalankanSelaras menyimpan AST Stmt mentah, bukan bytecode -- lihat
//    catatan di instr_ke_json di bawah).
// =====================================================================

fn value_ke_json(v: &Value) -> serde_json::Value {
    use serde_json::json;
    match v {
        Value::Angka(n) => json!({"t": "Angka", "v": n}),
        Value::Desimal(f) => json!({"t": "Desimal", "v": f}),
        Value::Teks(s) => json!({"t": "Teks", "v": s.as_ref()}),
        Value::Bool(b) => json!({"t": "Bool", "v": b}),
        Value::Daftar(items) => json!({"t": "Daftar", "v": items.iter().map(value_ke_json).collect::<Vec<_>>()}),
        // Degradasi ke format "Daftar" biasa -- isoteri-vm.js (jalur web/WASM) tidak perlu tahu
        // apa-apa soal representasi flat internal ini; outputnya harus byte-identik dengan
        // seolah-olah daftar ini masih Value::Daftar(Vec<Value::Angka/Desimal>) seperti sebelum
        // representasi flat ada.
        Value::DaftarAngka(items) => json!({"t": "Daftar", "v": items.iter().map(|n| json!({"t": "Angka", "v": n})).collect::<Vec<_>>()}),
        Value::DaftarDesimal(items) => json!({"t": "Daftar", "v": items.iter().map(|x| json!({"t": "Desimal", "v": x})).collect::<Vec<_>>()}),
        Value::Peta(entries) => json!({
            "t": "Peta",
            "v": entries.iter().map(|(k, v)| json!([k.as_ref(), value_ke_json(v)])).collect::<Vec<_>>()
        }),
        Value::Kosong => json!({"t": "Kosong"}),
        Value::Instans(nama, entries) => json!({
            "t": "Instans",
            "nama": nama.as_ref(),
            "v": entries.iter().map(|(k, v)| json!([k.as_ref(), value_ke_json(v)])).collect::<Vec<_>>()
        }),
        Value::Fungsi(nf) => json!({
            "t": "Fungsi",
            "idx": nf.idx,
            "tangkapan": nf.tangkapan.iter().map(value_ke_json).collect::<Vec<_>>()
        }),
    }
}

fn binop_ke_str(op: BinOp) -> &'static str {
    use BinOp::*;
    match op {
        Tambah => "Tambah", Kurang => "Kurang", Kali => "Kali", Bagi => "Bagi", Modulo => "Modulo",
        SamaDengan => "SamaDengan", TidakSama => "TidakSama",
        LebihBesar => "LebihBesar", LebihBesarSama => "LebihBesarSama",
        LebihKecil => "LebihKecil", LebihKecilSama => "LebihKecilSama",
        Dan => "Dan", Atau => "Atau",
    }
}

/// Satu instruksi -> array JSON `[opcode, ...operan]`, dipilih di atas object bernama
/// field supaya payload-nya ringkas (ribuan instruksi per program bukan hal aneh).
/// Err khusus untuk JalankanSelaras: instruksi ini menyimpan AST Stmt MENTAH (bukan
/// bytecode -- lihat komentar di enum Instr), jadi gak ada cara aman men-dump-nya jadi
/// bytecode datar tanpa menduplikasi seluruh interpreter Stmt di sisi JS. 'ulang selaras'
/// karena itu belum didukung di web runtime -- lihat runtime/web/README.md.
fn instr_ke_json(instr: &Instr) -> Result<serde_json::Value, String> {
    use serde_json::json;
    Ok(match instr {
        Instr::TandaiBaris(n) => json!(["TandaiBaris", n]),
        Instr::PushK(i) => json!(["PushK", i]),
        Instr::LoadGlobal(s) => json!(["LoadGlobal", s]),
        Instr::StoreGlobal(s) => json!(["StoreGlobal", s]),
        Instr::LoadLocal(s) => json!(["LoadLocal", s]),
        Instr::StoreLocal(s) => json!(["StoreLocal", s]),
        Instr::BinOp(op) => json!(["BinOp", binop_ke_str(*op)]),
        Instr::Lompat(t) => json!(["Lompat", t]),
        Instr::LompatJikaSalah(t) => json!(["LompatJikaSalah", t]),
        Instr::Tidak => json!(["Tidak"]),
        Instr::MakeDaftar(n) => json!(["MakeDaftar", n]),
        Instr::MakePeta(kunci) => json!(["MakePeta", kunci.iter().map(|k| k.as_ref()).collect::<Vec<&str>>()]),
        Instr::Indeks => json!(["Indeks"]),
        Instr::IndeksTahanIdx => json!(["IndeksTahanIdx"]),
        Instr::SetIndeks => json!(["SetIndeks"]),
        Instr::AmbilField(f) => json!(["AmbilField", f]),
        Instr::BuatInstans(nama, fields) => json!(["BuatInstans", nama.as_ref(), fields.iter().map(|k| k.as_ref()).collect::<Vec<&str>>()]),
        Instr::SetField(f) => json!(["SetField", f]),
        Instr::TambahkanLokal(s) => json!(["TambahkanLokal", s]),
        Instr::TambahkanGlobal(s) => json!(["TambahkanGlobal", s]),
        Instr::Dup => json!(["Dup"]),
        Instr::Tampilkan => json!(["Tampilkan"]),
        Instr::Pop => json!(["Pop"]),
        Instr::PanggilFungsi(idx, argc) => json!(["PanggilFungsi", idx, argc]),
        Instr::PanggilBawaan(nama, argc) => json!(["PanggilBawaan", nama, argc]),
        Instr::BuatFungsi(idx, n) => json!(["BuatFungsi", idx, n]),
        Instr::PanggilNilai(argc) => json!(["PanggilNilai", argc]),
        Instr::IterMulai => json!(["IterMulai"]),
        Instr::IterLanjutLocal(slot, target) => json!(["IterLanjutLocal", slot, target]),
        Instr::IterLanjutGlobal(slot, target) => json!(["IterLanjutGlobal", slot, target]),
        Instr::JalankanSelaras(..) => {
            return Err("'ulang selaras' belum didukung di web runtime (lihat runtime/web/README.md) -- pakai 'ulang setiap' biasa kalau butuh jalan di browser.".to_string());
        }
        Instr::MulaiCobaLocal(target, slot) => json!(["MulaiCobaLocal", target, slot]),
        Instr::MulaiCobaGlobal(target, slot) => json!(["MulaiCobaGlobal", target, slot]),
        Instr::SelesaiCoba => json!(["SelesaiCoba"]),
        Instr::TutupHandler => json!(["TutupHandler"]),
        Instr::Kembalikan => json!(["Kembalikan"]),
    })
}

fn kode_ke_json(kode: &[Instr]) -> Result<serde_json::Value, String> {
    kode.iter().map(instr_ke_json).collect::<Result<Vec<_>, _>>().map(serde_json::Value::Array)
}

/// Pipeline resolve -> compile TANPA jalankan JIT (native fn pointer gak relevan buat
/// ekspor -- kode bytecode fallback-nya, `VMFungsi::kode`, selalu ada apa pun status JIT-nya,
/// jadi cukup dump itu; browser konsisten pakai jalur bytecode biasa untuk semua fungsi).
pub fn ekspor_json_dari_sumber(sumber: &str) -> Result<String, String> {
    let mut lexer = Lexer::new(sumber);
    let tokens = lexer.tokenize().map_err(|e| format!("Kesalahan Lexer: {}", e))?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().map_err(|e| format!("Kesalahan Parser: {}", e))?;
    ekspor_json_dari_program(program)
}

/// Inti ekspor_json_dari_sumber, dipisah supaya bisa dipakai dari Vec<Stmt> yang sudah
/// melewati ekspansi_muat (dan rewrite alias) -- dipakai ekspor_json_dari_berkas.
fn ekspor_json_dari_program(program: Vec<(usize, Stmt)>) -> Result<String, String> {
    let mut resolver = Resolver::new();
    let top_level = resolver.resolve_top(&program).map_err(|e| format!("Kesalahan Kompilasi: {}", e))?;
    let top_level = optimisasi_blok(top_level);
    for cf in resolver.fungsi_out.values_mut() {
        if let Some(cf) = Rc::get_mut(cf) {
            cf.body = optimisasi_blok(std::mem::take(&mut cf.body));
        }
    }

    let mut nama_fungsi: Vec<String> = resolver.fungsi_out.keys().cloned().collect();
    nama_fungsi.sort(); // deterministik: HashMap tidak menjamin urutan iterasi konsisten antar-run,
    // jadi kalau tidak diurutkan, urutan fungsi (dan index-nya) di bundel .isoweb.json bisa
    // berbeda-beda tiap kali di-compile ulang walau source-nya sama persis (nondeterminism
    // bawaan, ditemukan sewaktu verifikasi representasi Daftar flat -- lihat ROADMAP.md).
    let fungsi_index: HashMap<String, usize> = nama_fungsi.iter().enumerate().map(|(i, n)| (n.clone(), i)).collect();

    let mut compiler = Compiler::new(fungsi_index.clone());
    let top_kode = compiler.compile_top(&top_level);

    let mut fungsi_json = Vec::with_capacity(nama_fungsi.len());
    for nama in &nama_fungsi {
        let cf = resolver.fungsi_out.get(nama).unwrap();
        let vmf = compiler.compile_fungsi(cf);
        let param_flat_json: Vec<serde_json::Value> = vmf.param_flat.iter()
            .map(|pf| match pf {
                Some(field_urut) => serde_json::Value::Array(field_urut.iter().map(|s| serde_json::Value::String(s.clone())).collect()),
                None => serde_json::Value::Null,
            })
            .collect();
        fungsi_json.push(serde_json::json!({
            "nama": nama,
            "param_count": vmf.param_count,
            "local_slot_count": vmf.local_slot_count,
            "kode": kode_ke_json(&vmf.kode).map_err(|e| format!("Fungsi \"{}\": {}", nama, e))?,
            "param_flat": param_flat_json,
        }));
    }

    let keluaran = serde_json::json!({
        "format": "isoteri-web-bytecode-v1",
        "global_slot_count": resolver.global_count,
        "konstanta": compiler.konstanta.iter().map(value_ke_json).collect::<Vec<_>>(),
        "nama_ke_indeks": fungsi_index,
        "fungsi": fungsi_json,
        "top_kode": kode_ke_json(&top_kode).map_err(|e| format!("Program utama: {}", e))?,
    });

    serde_json::to_string_pretty(&keluaran).map_err(|e| format!("Gagal menyusun JSON: {}", e))
}

/// Satu-satunya jalur pemuatan modul (parse + ekspansi_muat + deteksi tabrakan nama + rewrite
/// panggilan lewat alias) -- dipakai SEMUA entry point yang baca dari berkas (jalankan_berkas,
/// jalankan_berkas_via_ir, ekspor_json_dari_berkas). SEBELUM ini, ada DUA implementasi terpisah
/// (ekspansi_muat berbasis AST buat jalankan_berkas, kumpulkan_sumber_gabungan berbasis TEKS
/// MENTAH buat dua entry point lain) yang bisa diam-diam berbeda perilaku -- ditemukan sewaktu
/// menambah fitur alias modul ini: kumpulkan_sumber_gabungan (pencocokan baris tekstual, tidak
/// paham AST) langsung membuang bagian "sebagai alias" tanpa merasa perlu tahu artinya, jadi
/// fitur alias TIDAK akan bekerja sama sekali di jalur IR/JIT dan ekspor-web kalau kedua
/// implementasi ini dibiarkan terpisah. Disatukan supaya kelakuan modul (termasuk alias) IDENTIK
/// di semua jalur, dan supaya perbaikan/fitur modul ke depan cukup ditulis SEKALI.
pub fn program_dari_berkas(path: &str) -> Result<Vec<(usize, Stmt)>, String> {
    let sumber = fs::read_to_string(path).map_err(|e| format!("Tidak bisa membaca berkas \"{}\": {}", path, e))?;
    let mut lexer = Lexer::new(&sumber);
    let tokens = lexer.tokenize().map_err(|e| format!("Kesalahan Lexer: {}", e))?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().map_err(|e| format!("Kesalahan Parser: {}", e))?;

    let entry_path = std::path::Path::new(path);
    let entry_dir = entry_path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| std::path::PathBuf::from("."));
    let entry_label = entry_path.display().to_string();
    let mut sudah_dimuat = std::collections::HashSet::new();
    if let Ok(kanon) = fs::canonicalize(entry_path) { sudah_dimuat.insert(kanon); }
    let mut alias_dikenal = std::collections::HashSet::new();
    let program_berlabel = ekspansi_muat(program, &entry_label, &entry_dir, &mut sudah_dimuat, &mut alias_dikenal)
        .map_err(|e| format!("Kesalahan Muat: {}", e))?;
    cek_tabrakan_nama(&program_berlabel).map_err(|e| format!("Kesalahan Muat: {}", e))?;

    let program: Vec<(usize, Stmt)> = program_berlabel.into_iter()
        .map(|(baris, s, _)| (baris, tulis_ulang_panggil_alias(s, &alias_dikenal)))
        .collect();
    Ok(program)
}

/// Terjemahkan 'alias.fungsi(args)' (di-parse jadi Expr::PanggilMetode(Ident(alias), fungsi,
/// args) karena parser gak tahu 'alias' itu alias modul atau sekadar variabel biasa) jadi
/// panggilan fungsi langsung ke nama yang sudah di-mangle ("alias.fungsi") -- HANYA kalau base-
/// nya persis satu Ident yang cocok nama alias yang dikenal. PanggilMetode lain (base bukan Ident
/// polos, atau Ident tapi bukan alias) DIBIARKAN apa adanya -- itu artinya "panggil NILAI di
/// field itu sebagai fungsi" (mis. closure disimpan di field bentuk), ditangani generik di
/// resolver/compiler lewat CExpr::PanggilNilai, bukan di sini.
fn tulis_ulang_panggil_alias(s: Stmt, alias_dikenal: &std::collections::HashSet<String>) -> Stmt {
    fn expr(e: Expr, set: &std::collections::HashSet<String>) -> Expr {
        match e {
            Expr::PanggilMetode(base, nama, args) => {
                let args_baru: Vec<Expr> = args.into_iter().map(|a| expr(a, set)).collect();
                if let Expr::Ident(alias) = base.as_ref() {
                    if set.contains(alias) {
                        return Expr::Panggil(format!("__modul_{}__{}", alias, nama), args_baru);
                    }
                }
                Expr::PanggilMetode(Box::new(expr(*base, set)), nama, args_baru)
            }
            Expr::Panggil(nama, args) => Expr::Panggil(nama, args.into_iter().map(|a| expr(a, set)).collect()),
            Expr::Binary(l, op, r) => Expr::Binary(Box::new(expr(*l, set)), op, Box::new(expr(*r, set))),
            Expr::Daftar(items) => Expr::Daftar(items.into_iter().map(|i| expr(i, set)).collect()),
            Expr::Peta(entries) => Expr::Peta(entries.into_iter().map(|(k, v)| (k, expr(v, set))).collect()),
            Expr::Indeks(t, i) => Expr::Indeks(Box::new(expr(*t, set)), Box::new(expr(*i, set))),
            Expr::Field(t, f) => Expr::Field(Box::new(expr(*t, set)), f),
            Expr::Tidak(e) => Expr::Tidak(Box::new(expr(*e, set))),
            Expr::BentukLiteral(nama, entries) => Expr::BentukLiteral(nama, entries.into_iter().map(|(k, v)| (k, expr(v, set))).collect()),
            Expr::FungsiLiteral(params, body) => Expr::FungsiLiteral(params, body.into_iter().map(|(b, st)| (b, stmt(st, set))).collect()),
            lain => lain,
        }
    }
    fn stmt(s: Stmt, set: &std::collections::HashSet<String>) -> Stmt {
        match s {
            Stmt::FungsiDef(nama, params, body) => Stmt::FungsiDef(nama, params, body.into_iter().map(|(b, st)| (b, stmt(st, set))).collect()),
            Stmt::Kalau(c, tb, eb) => Stmt::Kalau(expr(c, set), tb.into_iter().map(|(b, st)| (b, stmt(st, set))).collect(), eb.map(|blk| blk.into_iter().map(|(b, st)| (b, stmt(st, set))).collect())),
            Stmt::Ulang(c, body) => Stmt::Ulang(expr(c, set), body.into_iter().map(|(b, st)| (b, stmt(st, set))).collect()),
            Stmt::UlangSetiap(v, e, body) => Stmt::UlangSetiap(v, expr(e, set), body.into_iter().map(|(b, st)| (b, stmt(st, set))).collect()),
            Stmt::UlangSelaras(v, e, body) => Stmt::UlangSelaras(v, expr(e, set), body.into_iter().map(|(b, st)| (b, stmt(st, set))).collect()),
            Stmt::Coba(tb, v, cb) => Stmt::Coba(tb.into_iter().map(|(b, st)| (b, stmt(st, set))).collect(), v, cb.into_iter().map(|(b, st)| (b, stmt(st, set))).collect()),
            Stmt::Ingat(nama, tipe, e) => Stmt::Ingat(nama, tipe, expr(e, set)),
            Stmt::Ubah(nama, e) => Stmt::Ubah(nama, expr(e, set)),
            Stmt::UbahField(nama, jalur, e) => Stmt::UbahField(nama, jalur, expr(e, set)),
            Stmt::UbahJalur(nama, jalur, e) => Stmt::UbahJalur(nama, jalur.into_iter().map(|j| match j { Jalur::Indeks(ie) => Jalur::Indeks(expr(ie, set)), lain => lain }).collect(), expr(e, set)),
            Stmt::Tampilkan(e) => Stmt::Tampilkan(expr(e, set)),
            Stmt::Kembalikan(e) => Stmt::Kembalikan(expr(e, set)),
            Stmt::EkspresiStmt(e) => Stmt::EkspresiStmt(expr(e, set)),
            lain => lain,
        }
    }
    stmt(s, alias_dikenal)
}

pub fn ekspor_json_dari_berkas(path: &str) -> Result<String, String> {
    ekspor_json_dari_program(program_dari_berkas(path)?)
}
