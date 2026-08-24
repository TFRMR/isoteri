// Workload "validasi_petani" -- versi Node.js, logika & dataset generator
// harus identik dengan isoteri/validasi_petani.iso dan python/validasi_petani.py
// supaya perbandingan adil. Lihat README.md di folder ini.

function validasiPetani(data) {
  if (!("nama" in data) || !("lahan" in data) || !("hasil_panen" in data)) {
    return "Field wajib hilang -- pastikan nama, lahan, dan hasil_panen semua diisi.";
  }
  const nama = data.nama;
  const lahan = data.lahan;
  const hasilPanen = data.hasil_panen;

  if (nama === "") {
    return "Nama tidak boleh kosong.";
  }
  if (nama.length > 100) {
    return "Nama terlalu panjang (maksimal 100 karakter).";
  }
  if (lahan <= 0) {
    return "Luas lahan harus lebih dari 0.";
  }
  if (lahan > 10000) {
    return "Luas lahan tidak masuk akal (maksimal 10.000 hektar).";
  }
  if (hasilPanen < 0) {
    return "Hasil panen tidak boleh negatif.";
  }
  return "";
}

function buatData(i) {
  const sisa = i % 5;
  if (sisa === 0) {
    return { nama: "Petani Menoreh", lahan: 2.5, hasil_panen: 12.0 };
  }
  if (sisa === 1) {
    return { nama: "", lahan: 2.5, hasil_panen: 12.0 };
  }
  if (sisa === 2) {
    return { nama: "Petani Menoreh", lahan: -1, hasil_panen: 12.0 };
  }
  if (sisa === 3) {
    return { nama: "Petani Menoreh", lahan: 99999, hasil_panen: 12.0 };
  }
  return { nama: "Petani Menoreh", lahan: 2.5 };
}

const n = 500000;
let jumlahValid = 0;
for (let i = 0; i < n; i++) {
  const data = buatData(i);
  const pesan = validasiPetani(data);
  if (pesan === "") {
    jumlahValid++;
  }
}
console.log(jumlahValid);
