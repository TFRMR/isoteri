# Hasil Benchmark Head-to-Head

Wall-clock end-to-end per proses (termasuk startup interpreter/runtime), median dari beberapa sampel setelah 1 run pemanasan dibuang. Lihat README.md untuk metodologi & keterbatasan lengkap.


## validasi_petani

| Bahasa | Median | Min | Max | Stdev |
|---|---:|---:|---:|---:|
| Isoteri (AOT) | 1031.44ms | 980.85ms | 1089.63ms | 32.30ms |
| Node.js | 35.70ms | 31.80ms | 42.21ms | 3.21ms |
| Python | 169.91ms | 160.04ms | 173.81ms | 4.43ms |

- Node.js 28.9x lebih cepat dari Isoteri
- Python 6.1x lebih cepat dari Isoteri

## fib_rekursif

| Bahasa | Median | Min | Max | Stdev |
|---|---:|---:|---:|---:|
| Isoteri (AOT) | 17.73ms | 17.65ms | 17.94ms | 0.11ms |
| Node.js | 44.40ms | 41.42ms | 46.86ms | 1.60ms |
| Python | 210.61ms | 205.87ms | 217.84ms | 3.81ms |

- Isoteri 2.5x lebih cepat dari Node.js
- Isoteri 11.9x lebih cepat dari Python

## daftar_operasi

| Bahasa | Median | Min | Max | Stdev |
|---|---:|---:|---:|---:|
| Isoteri (AOT) | 16.86ms | 15.57ms | 17.30ms | 0.57ms |
| Node.js | 27.08ms | 24.70ms | 28.78ms | 1.18ms |
| Python | 14.20ms | 13.47ms | 14.82ms | 0.60ms |

- Isoteri 1.6x lebih cepat dari Node.js
- Python 1.2x lebih cepat dari Isoteri
