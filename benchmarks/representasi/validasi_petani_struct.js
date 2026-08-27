// Padanan Node.js untuk validasi_petani_struct.iso -- struct/object literal
// dengan shape TETAP (bukan dict dinamis) supaya V8 bisa pakai hidden
// class yang sama tiap kali (best-case buat V8, bukan cuma "adil").

function validasiPetaniStruct(data) {
  if (data.namaKosong === 1) return 1;
  if (data.lahan <= 0) return 1;
  if (data.lahan > 10000) return 1;
  if (data.hasilPanen < 0) return 1;
  return 0;
}

function validasiSatu(i) {
  const sisa = i % 5;
  if (sisa === 0) return validasiPetaniStruct({ namaKosong: 0, lahan: 2.5, hasilPanen: 12.0 });
  if (sisa === 1) return validasiPetaniStruct({ namaKosong: 1, lahan: 2.5, hasilPanen: 12.0 });
  if (sisa === 2) return validasiPetaniStruct({ namaKosong: 0, lahan: -1, hasilPanen: 12.0 });
  if (sisa === 3) return validasiPetaniStruct({ namaKosong: 0, lahan: 99999, hasilPanen: 12.0 });
  return validasiPetaniStruct({ namaKosong: 0, lahan: 2.5, hasilPanen: -1 });
}

const n = 500000;
let jumlahValid = 0;
for (let i = 0; i < n; i++) {
  if (validasiSatu(i) === 0) jumlahValid++;
}
console.log(jumlahValid);
