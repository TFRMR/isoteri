# Hasil Benchmark Head-to-Head

Wall-clock end-to-end per proses (termasuk startup interpreter/runtime), median dari beberapa sampel setelah 1 run pemanasan dibuang. Lihat README.md untuk metodologi & keterbatasan lengkap.


## validasi_petani

| Bahasa | Median | Min | Max | Stdev |
|---|---:|---:|---:|---:|
| Isoteri (AOT) | 1222.35ms | 1205.94ms | 1247.63ms | 15.88ms |
| Node.js | 40.08ms | 37.50ms | 47.69ms | 3.33ms |
| Python | 192.40ms | 188.35ms | 196.51ms | 3.25ms |

- Node.js 30.5x lebih cepat dari Isoteri
- Python 6.4x lebih cepat dari Isoteri

## fib_rekursif

| Bahasa | Median | Min | Max | Stdev |
|---|---:|---:|---:|---:|
| Isoteri (AOT) | 20.94ms | 20.86ms | 21.24ms | 0.16ms |
| Node.js | 53.97ms | 50.83ms | 61.94ms | 3.74ms |
| Python | 251.96ms | 243.52ms | 271.77ms | 9.47ms |

- Isoteri 2.6x lebih cepat dari Node.js
- Isoteri 12.0x lebih cepat dari Python

## daftar_operasi

| Bahasa | Median | Min | Max | Stdev |
|---|---:|---:|---:|---:|
| Isoteri (AOT) | 4196.78ms | 4043.88ms | 4319.55ms | 112.67ms |
| Node.js | 32.21ms | 30.15ms | 33.14ms | 0.99ms |
| Python | 16.12ms | 15.92ms | 18.07ms | 0.86ms |

- Node.js 130.3x lebih cepat dari Isoteri
- Python 260.3x lebih cepat dari Isoteri
