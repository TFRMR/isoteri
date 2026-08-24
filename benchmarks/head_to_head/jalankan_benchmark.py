#!/usr/bin/env python3
"""Harness benchmark head-to-head: Isoteri (AOT native) vs Node.js vs Python.

Metodologi:
- Tiap kombinasi (workload, bahasa) dijalankan SAMPEL kali sebagai proses
  terpisah (subprocess), diukur wall-clock end-to-end (termasuk startup
  proses/interpreter -- ini SENGAJA, karena mencerminkan skenario request
  singkat di backend, bukan cuma throughput loop panas).
- 1 run pemanasan dibuang sebelum sampling (menghindari efek cold-cache
  filesystem/OS, bukan buat "menghangatkan" JIT -- proses baru tiap run
  jadi JIT/interpreter mulai dingin lagi tiap kali, ini realistis untuk
  proses CLI berumur pendek).
- Output tiap run diverifikasi SAMA persis lintas bahasa -- kalau beda,
  benchmark dianggap tidak valid dan dicatat sebagai galat, bukan diam-diam
  diabaikan.
- Statistik: median (tahan outlier) + min/max + stdev, dari SAMPEL run.

Pakai: python3 jalankan_benchmark.py [--sampel N]
"""
import json
import statistics
import subprocess
import sys
import time
from pathlib import Path

DIR = Path(__file__).parent
SAMPEL_DEFAULT = 10

# (nama_workload, {bahasa: (cmd_list, hasil_diharapkan)})
WORKLOADS = {
    "validasi_petani": {
        "Isoteri (AOT)": (["isoteri/validasi_petani_aot"], "100000"),
        "Node.js": (["node", "node/validasi_petani.js"], "100000"),
        "Python": (["python3", "python/validasi_petani.py"], "100000"),
    },
    "fib_rekursif": {
        "Isoteri (AOT)": (["isoteri/fib_rekursif_aot"], "2178309"),
        "Node.js": (["node", "node/fib_rekursif.js"], "2178309"),
        "Python": (["python3", "python/fib_rekursif.py"], "2178309"),
    },
    "daftar_operasi": {
        "Isoteri (AOT)": (["isoteri/daftar_operasi_aot"], "20930000000"),
        "Node.js": (["node", "node/daftar_operasi.js"], "20930000000"),
        "Python": (["python3", "python/daftar_operasi.py"], "20930000000"),
    },
}


def jalankan_sekali(cmd):
    mulai = time.perf_counter()
    hasil = subprocess.run(cmd, cwd=DIR, capture_output=True, text=True, timeout=120)
    selesai = time.perf_counter()
    return selesai - mulai, hasil.stdout.strip(), hasil.returncode


def benchmark_satu(cmd, hasil_diharapkan, sampel):
    # 1 run pemanasan, dibuang dari statistik
    _, keluaran, kode = jalankan_sekali(cmd)
    if kode != 0:
        return None, f"exit code {kode} saat pemanasan"
    if keluaran != hasil_diharapkan:
        return None, f"output salah: dapat {keluaran!r}, harap {hasil_diharapkan!r}"

    waktu = []
    for _ in range(sampel):
        w, keluaran, kode = jalankan_sekali(cmd)
        if kode != 0 or keluaran != hasil_diharapkan:
            return None, f"output tidak konsisten pada sampling (dapat {keluaran!r})"
        waktu.append(w)
    return waktu, None


def main():
    sampel = SAMPEL_DEFAULT
    if "--sampel" in sys.argv:
        sampel = int(sys.argv[sys.argv.index("--sampel") + 1])

    laporan = {}
    print(f"Menjalankan benchmark ({sampel} sampel per kombinasi, + 1 pemanasan dibuang)...\n")

    for nama_workload, bahasa_dict in WORKLOADS.items():
        print(f"=== {nama_workload} ===")
        laporan[nama_workload] = {}
        for bahasa, (cmd, harap) in bahasa_dict.items():
            waktu, galat = benchmark_satu(cmd, harap, sampel)
            if galat:
                print(f"  {bahasa:20s} GAGAL: {galat}")
                laporan[nama_workload][bahasa] = {"galat": galat}
                continue
            median = statistics.median(waktu)
            stdev = statistics.stdev(waktu) if len(waktu) > 1 else 0.0
            print(
                f"  {bahasa:20s} median={median*1000:9.2f}ms  "
                f"min={min(waktu)*1000:9.2f}ms  max={max(waktu)*1000:9.2f}ms  "
                f"stdev={stdev*1000:7.2f}ms"
            )
            laporan[nama_workload][bahasa] = {
                "median_ms": median * 1000,
                "min_ms": min(waktu) * 1000,
                "max_ms": max(waktu) * 1000,
                "stdev_ms": stdev * 1000,
                "sampel_ms": [w * 1000 for w in waktu],
            }
        print()

    out_json = DIR / "hasil" / "hasil_mentah.json"
    out_json.write_text(json.dumps(laporan, indent=2, ensure_ascii=False))
    print(f"Hasil mentah disimpan ke {out_json}")

    tulis_markdown(laporan, DIR / "hasil" / "HASIL.md")
    print(f"Laporan Markdown disimpan ke {DIR / 'hasil' / 'HASIL.md'}")


def tulis_markdown(laporan, path_keluaran):
    baris = ["# Hasil Benchmark Head-to-Head\n"]
    baris.append(
        "Wall-clock end-to-end per proses (termasuk startup interpreter/runtime), "
        "median dari beberapa sampel setelah 1 run pemanasan dibuang. "
        "Lihat README.md untuk metodologi & keterbatasan lengkap.\n"
    )
    for nama_workload, bahasa_dict in laporan.items():
        baris.append(f"\n## {nama_workload}\n")
        baris.append("| Bahasa | Median | Min | Max | Stdev |")
        baris.append("|---|---:|---:|---:|---:|")
        for bahasa, data in bahasa_dict.items():
            if "galat" in data:
                baris.append(f"| {bahasa} | GAGAL: {data['galat']} | | | |")
                continue
            baris.append(
                f"| {bahasa} | {data['median_ms']:.2f}ms | {data['min_ms']:.2f}ms | "
                f"{data['max_ms']:.2f}ms | {data['stdev_ms']:.2f}ms |"
            )
        # Hitung rasio kalau Isoteri ada & berhasil
        isoteri = bahasa_dict.get("Isoteri (AOT)", {})
        if "median_ms" in isoteri:
            baris.append("")
            for bahasa, data in bahasa_dict.items():
                if bahasa == "Isoteri (AOT)" or "median_ms" not in data:
                    continue
                rasio = data["median_ms"] / isoteri["median_ms"]
                baris.append(f"- Isoteri {rasio:.1f}x lebih cepat dari {bahasa}" if rasio >= 1
                             else f"- {bahasa} {1/rasio:.1f}x lebih cepat dari Isoteri")
    path_keluaran.write_text("\n".join(baris) + "\n")


if __name__ == "__main__":
    main()
