#!/usr/bin/env python3
"""Benchmark python-webhook: stdlib-only CGI (no Flask/WSGI — lowest overhead on shared Apache CGI)."""

from __future__ import annotations

import json
import os
import sys

WEBHOOK_SECRET = "bench-secret"


def respond(status: int, reason: str, content_type: str, body: bytes) -> None:
    out = sys.stdout.buffer
    out.write(f"Status: {status} {reason}\r\n".encode())
    out.write(f"Content-Type: {content_type}\r\n".encode())
    out.write(f"Content-Length: {len(body)}\r\n".encode())
    out.write(b"\r\n")
    out.write(body)


def main() -> None:
    if os.environ.get("REQUEST_METHOD", "") != "POST":
        respond(405, "Method Not Allowed", "text/plain", b"method not allowed\n")
        return

    if os.environ.get("HTTP_X_WEBHOOK_SECRET", "") != WEBHOOK_SECRET:
        respond(401, "Unauthorized", "text/plain", b"unauthorized\n")
        return

    try:
        length = int(os.environ.get("CONTENT_LENGTH", "0"))
    except ValueError:
        respond(400, "Bad Request", "text/plain", b"bad request\n")
        return

    raw = sys.stdin.buffer.read(length)
    try:
        evt = json.loads(raw)
        event_id = evt["id"]
    except (json.JSONDecodeError, KeyError, TypeError):
        respond(400, "Bad Request", "text/plain", b"bad request\n")
        return

    body = json.dumps({"received": True, "id": event_id}).encode()
    respond(200, "OK", "application/json", body)


if __name__ == "__main__":
    main()
