//! Wrapper wasm-bindgen TIPIS di atas compiler Isoteri asli. TIDAK ADA logika
//! kompilasi apa pun ditulis ulang di sini -- satu-satunya isi berarti file
//! ini adalah memanggil `isoteri::ekspor_json_dari_sumber()` langsung, fungsi
//! PERSIS yang sama dipakai CLI native (`isoteri ekspor-web`). Ini disengaja:
//! proyek Isoteri sudah berkali-kali (lihat KETERBATASAN.md, docs/IR.md)
//! menemukan & memperbaiki bug yang muncul justru karena ada DUA implementasi
//! paralel dari hal yang sama (bytecode VM vs JIT) yang diam-diam beda
//! perilaku. Reimplementasi compiler dalam bentuk lain di sini (mis. parser
//! JS terpisah) akan membuka kelas bug yang SAMA PERSIS lagi -- jadi jangan.

use wasm_bindgen::prelude::*;

/// Kompilasi source Isoteri (Teks mentah, isi file .iso) langsung jadi bundle
/// bytecode JSON (Teks) yang siap dipakai `IsoteriVM` (lihat runtime/web/isoteri-vm.js)
/// -- SAMA PERSIS hasilnya dengan `isoteri ekspor-web sumber.iso -o bundle.json`
/// versi CLI, tapi jalan langsung di browser tanpa proses compile terpisah.
///
/// Pemakaian dari JS (setelah wasm-pack build --target web, lihat README.md):
/// ```js
/// import init, { kompilasi } from "./pkg/isoteri_wasm.js";
/// await init();
/// try {
///   const bundelJson = kompilasi(sumberIsoteri);
///   const vm = new IsoteriVM(JSON.parse(bundelJson));
///   vm.jalankan();
/// } catch (pesanError) {
///   console.error(pesanError); // Teks error Isoteri asli (Lexer/Parser/Kompilasi)
/// }
/// ```
#[wasm_bindgen]
pub fn kompilasi(sumber: &str) -> Result<String, String> {
    isoteri::ekspor_json_dari_sumber(sumber)
}

/// Versi crate isoteri-wasm ini -- buat sisi JS mengecek modul termuat & versi berapa
/// (mis. ditampilkan di UI Studio: "Compiler Isoteri (wasm) vX.X.X aktif").
#[wasm_bindgen]
pub fn versi() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kompilasi_menghasilkan_json_valid() {
        let hasil = kompilasi("tampilkan 1 + 2").expect("harus sukses kompilasi");
        assert!(hasil.contains("PushK"), "bundle JSON harus mengandung instruksi bytecode: {}", hasil);
        let _: serde_json::Value = serde_json::from_str(&hasil).expect("harus JSON valid");
    }

    #[test]
    fn kompilasi_error_sintaks_mengembalikan_pesan_jelas() {
        let hasil = kompilasi("ingat x = ");
        assert!(hasil.is_err());
    }

    #[test]
    fn versi_mengembalikan_string_tidak_kosong() {
        assert!(!versi().is_empty());
    }

    // CATATAN buat siapa pun yang extend crate ini: sebelum menambah fungsi publik baru,
    // verifikasi hasilnya BYTE-IDENTIK dengan `isoteri ekspor-web` versi CLI untuk source
    // yang sama (mis. `diff <(isoteri ekspor-web x.iso -o -) <(echo 'kompilasi("...")' lewat
    // wasm)`) -- ini sudah dilakukan manual sekali saat crate ini dibuat (hasilnya identik,
    // wajar karena keduanya manggil fungsi isoteri::ekspor_json_dari_sumber yang SAMA), dan
    // WAJIB diulang kalau logic kompilasi() di atas pernah disentuh lagi.
}
