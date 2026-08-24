"""Workload "daftar_operasi" -- versi Python. N SAMA dengan versi Isoteri
(n=20000), tapi build list pakai append() -- idiomatik Python, O(1)
amortized. Beda karakteristik build-list ini SENGAJA dibiarkan & dicatat
jujur di README.md, bukan dipaksa sama supaya "adil" secara artifisial."""


def konversi_ke_rupiah(kg):
    return kg * 5000


def di_atas_ambang(x):
    return x > 1000000


if __name__ == "__main__":
    n = 20000
    data = []
    for i in range(n):
        data.append(i % 500)

    nilai_rupiah = list(map(konversi_ke_rupiah, data))
    signifikan = list(filter(di_atas_ambang, nilai_rupiah))
    print(sum(signifikan))
