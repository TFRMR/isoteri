"""Workload "fib_rekursif" -- versi Python, N harus identik dengan
isoteri/fib_rekursif.iso dan node/fib_rekursif.js."""


def fib(n):
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)


if __name__ == "__main__":
    print(fib(32))
