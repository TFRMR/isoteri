# Hasil Benchmark Head-to-Head

Wall-clock end-to-end per proses (termasuk startup interpreter/runtime), median dari beberapa sampel setelah 1 run pemanasan dibuang. Lihat README.md untuk metodologi & keterbatasan lengkap.


## validasi_petani

| Bahasa | Median | Min | Max | Stdev |
|---|---:|---:|---:|---:|
| Isoteri (AOT) | 1210.99ms | 1194.92ms | 1236.10ms | 13.28ms |
| Node.js | 38.57ms | 35.88ms | 43.59ms | 2.47ms |
| Python | 188.16ms | 185.40ms | 196.92ms | 3.42ms |

- Node.js 31.4x lebih cepat dari Isoteri
- Python 6.4x lebih cepat dari Isoteri

## fib_rekursif

| Bahasa | Median | Min | Max | Stdev |
|---|---:|---:|---:|---:|
| Isoteri (AOT) | 20.66ms | 20.55ms | 22.03ms | 0.45ms |
| Node.js | 49.75ms | 47.94ms | 54.86ms | 2.07ms |
| Python | 244.08ms | 239.83ms | 247.65ms | 2.85ms |

- Isoteri 2.4x lebih cepat dari Node.js
- Isoteri 11.8x lebih cepat dari Python

## daftar_operasi

| Bahasa | Median | Min | Max | Stdev |
|---|---:|---:|---:|---:|
| Isoteri (AOT) | 20.00ms | 19.65ms | 20.54ms | 0.27ms |
| Node.js | 31.15ms | 28.44ms | 33.31ms | 1.78ms |
| Python | 17.05ms | 15.75ms | 18.48ms | 0.88ms |

- Isoteri 1.6x lebih cepat dari Node.js
- Python 1.2x lebih cepat dari Isoteri
