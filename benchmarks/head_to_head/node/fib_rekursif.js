// Workload "fib_rekursif" -- versi Node.js, N harus identik dengan
// isoteri/fib_rekursif.iso dan python/fib_rekursif.py.

function fib(n) {
  if (n <= 1) return n;
  return fib(n - 1) + fib(n - 2);
}

console.log(fib(32));
