"""Workload "validasi_petani" -- versi Python, logika & dataset generator
harus identik dengan isoteri/validasi_petani.iso dan node/validasi_petani.js
supaya perbandingan adil. Lihat README.md di folder ini."""


def validasi_petani(data):
    if "nama" not in data or "lahan" not in data or "hasil_panen" not in data:
        return "Field wajib hilang -- pastikan nama, lahan, dan hasil_panen semua diisi."
    nama = data["nama"]
    lahan = data["lahan"]
    hasil_panen = data["hasil_panen"]

    if nama == "":
        return "Nama tidak boleh kosong."
    if len(nama) > 100:
        return "Nama terlalu panjang (maksimal 100 karakter)."
    if lahan <= 0:
        return "Luas lahan harus lebih dari 0."
    if lahan > 10000:
        return "Luas lahan tidak masuk akal (maksimal 10.000 hektar)."
    if hasil_panen < 0:
        return "Hasil panen tidak boleh negatif."
    return ""


def buat_data(i):
    sisa = i % 5
    if sisa == 0:
        return {"nama": "Petani Menoreh", "lahan": 2.5, "hasil_panen": 12.0}
    if sisa == 1:
        return {"nama": "", "lahan": 2.5, "hasil_panen": 12.0}
    if sisa == 2:
        return {"nama": "Petani Menoreh", "lahan": -1, "hasil_panen": 12.0}
    if sisa == 3:
        return {"nama": "Petani Menoreh", "lahan": 99999, "hasil_panen": 12.0}
    return {"nama": "Petani Menoreh", "lahan": 2.5}


if __name__ == "__main__":
    n = 500000
    jumlah_valid = 0
    for i in range(n):
        data = buat_data(i)
        pesan = validasi_petani(data)
        if pesan == "":
            jumlah_valid += 1
    print(jumlah_valid)
