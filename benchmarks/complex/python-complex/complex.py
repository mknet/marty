#!/usr/bin/env python3
"""Complex benchmark: three CPU-heavy GET routes (stdlib CGI)."""

from __future__ import annotations

import json
import os
import sys
import time
from collections.abc import Callable
from typing import Any
from urllib.parse import parse_qs, unquote

PROCESS_START = time.perf_counter()
UNTIL_PROCESS_US: int | None = None

DEFAULT_PRIME_LIMIT = 400_000
DEFAULT_FIB_N = 42
DEFAULT_MATRIX_SIZE = 128
MAX_PRIME_LIMIT = 1_000_000
MAX_FIB_N = 50
MAX_MATRIX_SIZE = 200
SALT_MOD_PRIME = 5_000
SALT_MOD_MATRIX = 16
FIB_SEED = 42
FIB_REPEAT_BASE = 3_000
FIB_REPEAT_SALT_MOD = 7_000


def request_salt(explicit: int | None) -> int:
    if explicit is not None:
        return explicit
    return time.time_ns() % (2**62)


def effective_prime_limit(base: int, salt: int) -> int:
    return min(base + (salt % SALT_MOD_PRIME), MAX_PRIME_LIMIT)


def fibonacci_work(salt: int) -> int:
    repeats = FIB_REPEAT_BASE + (salt % FIB_REPEAT_SALT_MOD)
    acc = 0
    for _ in range(repeats):
        acc = (acc + fibonacci(FIB_SEED)) % (2**62)
    return acc


def effective_matrix_size(base: int, salt: int) -> int:
    return min(base + (salt % SALT_MOD_MATRIX), MAX_MATRIX_SIZE)


def prime_count(limit: int) -> int:
    if limit <= 2:
        return 0
    sieve = bytearray(b"\x01") * limit
    sieve[0] = sieve[1] = 0
    count = 0
    i = 2
    while i < limit:
        if sieve[i]:
            count += 1
            j = i * i
            while j < limit:
                sieve[j] = 0
                j += i
        i += 1
    return count


def fibonacci(n: int) -> int:
    if n == 0:
        return 0
    a, b = 0, 1
    for _ in range(1, n):
        a, b = b, a + b
    return b


def matrix_checksum(n: int) -> float:
    if n == 0:
        return 0.0
    size = n * n
    a = [float(i) * 0.001 for i in range(size)]
    b = [float(i) * 0.002 for i in range(size)]
    c = [0.0] * size
    for i in range(n):
        for j in range(n):
            total = 0.0
            for k in range(n):
                total += a[i * n + k] * b[k * n + j]
            c[i * n + j] = total
    return sum(c)


def bench_sent_us() -> int | None:
    raw = os.environ.get("HTTP_X_BENCH_SENT_US")
    if not raw:
        return None
    try:
        return int(raw)
    except ValueError:
        return None


def since_sent_us(sent: int) -> int:
    return int(time.time() * 1_000_000) - sent


def capture_until_process() -> None:
    global UNTIL_PROCESS_US
    sent = bench_sent_us()
    if sent is not None:
        UNTIL_PROCESS_US = since_sent_us(sent)


def elapsed_us() -> int:
    return int((time.perf_counter() - PROCESS_START) * 1_000_000)


def log_bench_pre_compute(route: str) -> None:
    if os.environ.get("BENCH_TIMING") != "1":
        return
    with open("/var/log/bench-timing/requests.log", "a", encoding="ascii") as f:
        f.write(f"python-complex\t{route}\t{elapsed_us()}\n")


def log_phase(profile: bool, phase: str) -> None:
    if not profile:
        return
    print(
        f"bench-timing lang=python-complex phase={phase} elapsed_us={elapsed_us()}",
        file=sys.stderr,
    )


def wants_profile() -> bool:
    qs = os.environ.get("QUERY_STRING", "")
    if not qs:
        return False
    return parse_qs(qs, keep_blank_values=True).get("profile", [""])[0] == "1"


def parse_salt() -> int | None:
    qs = os.environ.get("QUERY_STRING", "")
    if not qs:
        return None
    params = parse_qs(qs, keep_blank_values=True)
    raw = params.get("salt", [None])[0]
    if raw is None or raw == "":
        return None
    return int(raw)


def run_timed(route: str, salt: int, profile: bool, compute: Callable[[], Any]) -> None:
    handler_enter = elapsed_us()
    log_phase(profile, "handler_enter")
    pre_compute = elapsed_us()
    log_bench_pre_compute(route)
    until_compute = None
    sent = bench_sent_us()
    if sent is not None:
        until_compute = since_sent_us(sent)
    log_phase(profile, "pre_compute")
    t0 = time.perf_counter()
    result = compute()
    compute_us = int((time.perf_counter() - t0) * 1_000_000)
    log_phase(profile, "post_compute")
    total = elapsed_us()
    post_compute = total - pre_compute - compute_us
    prof = None
    if profile:
        prof = {
            "until_process_us": UNTIL_PROCESS_US,
            "until_compute_us": until_compute,
            "startup_us": handler_enter,
            "handler_setup_us": pre_compute - handler_enter,
            "compute_us": compute_us,
            "post_compute_us": post_compute,
            "total_us": total,
        }
        print(
            f"bench-timing lang=python-complex summary startup_us={handler_enter} "
            f"compute_us={compute_us} total_us={total}",
            file=sys.stderr,
        )
    respond_json(route, salt, result, prof)


def respond_json(route: str, salt: int, result: Any, profile: dict | None = None) -> None:
    payload: dict[str, Any] = {"route": route, "salt": salt, "result": result}
    if profile is not None:
        payload["profile"] = profile
    body = json.dumps(payload).encode()
    out = sys.stdout.buffer
    out.write(b"Status: 200 OK\r\n")
    out.write(b"Cache-Control: no-store, no-cache, must-revalidate\r\n")
    out.write(b"Pragma: no-cache\r\n")
    out.write(b"Content-Type: application/json\r\n")
    out.write(f"Content-Length: {len(body)}\r\n".encode())
    out.write(b"\r\n")
    out.write(body)


def respond(status: int, reason: str, body: bytes) -> None:
    out = sys.stdout.buffer
    out.write(f"Status: {status} {reason}\r\n".encode())
    out.write(b"Cache-Control: no-store\r\n")
    out.write(b"Content-Type: text/plain\r\n")
    out.write(f"Content-Length: {len(body)}\r\n".encode())
    out.write(b"\r\n")
    out.write(body)


def route_path() -> str:
    info = os.environ.get("PATH_INFO", "")
    if info:
        return unquote(info.strip("/"))
    uri = os.environ.get("REQUEST_URI", "")
    if "?" in uri:
        uri = uri.split("?", 1)[0]
    for marker in ("/python-complex/", "/marty-complex/", "/go-complex/"):
        if marker in uri:
            return unquote(uri.split(marker, 1)[1].strip("/"))
    return ""


def main() -> None:
    if os.environ.get("REQUEST_METHOD", "") != "GET":
        respond(405, "Method Not Allowed", b"method not allowed\n")
        return

    capture_until_process()
    profile = wants_profile()
    log_phase(profile, "pre_serve_cgi")
    salt = request_salt(parse_salt())
    parts = [p for p in route_path().split("/") if p]
    if not parts:
        respond(
            404,
            "Not Found",
            b"routes: /primes/{limit}, /fibonacci/{n}, /matrix/{size}\n",
        )
        return

    name = parts[0]
    arg = int(parts[1]) if len(parts) > 1 and parts[1].isdigit() else None

    if name == "primes":
        base = arg if arg is not None else DEFAULT_PRIME_LIMIT
        effective = effective_prime_limit(base, salt)
        run_timed("primes", salt, profile, lambda: prime_count(effective))
    elif name == "fibonacci":
        run_timed("fibonacci", salt, profile, lambda: fibonacci_work(salt))
    elif name == "matrix":
        base = arg if arg is not None else DEFAULT_MATRIX_SIZE
        effective = effective_matrix_size(base, salt)
        run_timed(
            "matrix",
            salt,
            profile,
            lambda: {"size": effective, "checksum": matrix_checksum(effective)},
        )
    else:
        respond(
            404,
            "Not Found",
            b"routes: /primes/{limit}, /fibonacci/{n}, /matrix/{size}\n",
        )


if __name__ == "__main__":
    main()
