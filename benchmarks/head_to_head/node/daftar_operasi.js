// Workload "daftar_operasi" -- versi Node.js. N SAMA dengan versi Isoteri
// (n=20000), tapi build array pakai push() -- idiomatik JS, O(1) amortized.
// Beda karakteristik build-list ini SENGAJA dibiarkan & dicatat jujur di
// README.md, bukan dipaksa sama supaya "adil" secara artifisial.

function konversiKeRupiah(kg) {
  return kg * 5000;
}

function diAtasAmbang(x) {
  return x > 1000000;
}

const n = 20000;
const data = [];
for (let i = 0; i < n; i++) {
  data.push(i % 500);
}

const nilaiRupiah = data.map(konversiKeRupiah);
const signifikan = nilaiRupiah.filter(diAtasAmbang);
const total = signifikan.reduce((a, b) => a + b, 0);
console.log(total);
