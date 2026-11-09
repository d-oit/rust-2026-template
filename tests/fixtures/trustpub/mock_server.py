#!/usr/bin/env python3
"""Offline crates.io API fixture for tests/trustpub_cli_test.sh."""

from __future__ import annotations

import json
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlparse

STATE = {
    "configs": {},
    "trustpub_only": {},
    "next_id": 1,
    "posts": 0,
    "deletes": 0,
    "requests": [],
}


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_args):  # silence request logging
        return

    def send_json(self, status, payload=None):
        body = b"" if payload is None else json.dumps(payload).encode()
        self.send_response(status)
        if payload is not None:
            self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        if body:
            self.wfile.write(body)

    def read_json(self):
        length = int(self.headers.get("Content-Length", "0"))
        return json.loads(self.rfile.read(length) or b"{}")

    def record(self):
        STATE["requests"].append((self.command, urlparse(self.path).path))

    def do_GET(self):
        self.record()
        parsed = urlparse(self.path)
        if parsed.path == "/__state":
            return self.send_json(200, STATE)
        if parsed.path == "/api/v1/trusted_publishing/github_configs":
            crate = parse_qs(parsed.query).get("crate", [""])[0]
            if crate == "unauth":
                return self.send_json(401, {"errors": [{"detail": "bad token"}]})
            if crate == "ratelimit":
                return self.send_json(429, {"errors": [{"detail": "slow down"}]})
            if crate == "servererr":
                return self.send_json(500, {"errors": [{"detail": "boom"}]})
            configs = STATE["configs"].get(crate, [])
            return self.send_json(
                200,
                {"github_configs": configs, "meta": {"total": len(configs), "next_page": None}},
            )
        if parsed.path.startswith("/api/v1/crates/"):
            parts = parsed.path.strip("/").split("/")
            crate = parts[3]
            if len(parts) == 4:
                return self.send_json(
                    200, {"crate": {"name": crate, "trustpub_only": STATE["trustpub_only"].get(crate, False)}}
                )
            return self.send_json(200 if crate in {"demo", "dep-lib", "regfail"} else 404)
        return self.send_json(404, {"errors": [{"detail": "unknown path"}]})

    def do_POST(self):
        self.record()
        parsed = urlparse(self.path)
        if parsed.path == "/__seed":
            payload = self.read_json()
            crate = payload["crate"]
            entry = payload["config"]
            entry.setdefault("id", STATE["next_id"])
            STATE["next_id"] += 1
            STATE["configs"].setdefault(crate, []).append(entry)
            return self.send_json(200, entry)
        if parsed.path == "/api/v1/trusted_publishing/github_configs":
            payload = self.read_json()
            config = payload["github_config"]
            if config["crate"] == "regfail":
                return self.send_json(500, {"errors": [{"detail": "registration failed"}]})
            entry = dict(config)
            entry["id"] = STATE["next_id"]
            STATE["next_id"] += 1
            entry.setdefault("created_at", "2026-01-01T00:00:00Z")
            STATE["configs"].setdefault(config["crate"], []).append(entry)
            STATE["posts"] += 1
            return self.send_json(200, {"github_config": entry})
        return self.send_json(404, {"errors": [{"detail": "unknown path"}]})

    def do_DELETE(self):
        self.record()
        parsed = urlparse(self.path)
        parts = parsed.path.strip("/").split("/")
        if len(parts) == 5 and parts[3] == "github_configs":
            config_id = int(parts[4])
            for crate, entries in STATE["configs"].items():
                kept = [item for item in entries if item["id"] != config_id]
                if len(kept) != len(entries):
                    STATE["configs"][crate] = kept
                    STATE["deletes"] += 1
                    return self.send_json(204)
        return self.send_json(404, {"errors": [{"detail": "not found"}]})

    def do_PATCH(self):
        self.record()
        parsed = urlparse(self.path)
        parts = parsed.path.strip("/").split("/")
        if len(parts) == 4 and parts[2] == "crates":
            crate = parts[3]
            STATE["trustpub_only"][crate] = self.read_json()["crate"]["trustpub_only"]
            return self.send_json(200, {"crate": {"name": crate}})
        return self.send_json(404, {"errors": [{"detail": "unknown path"}]})


def main() -> int:
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 0
    server = ThreadingHTTPServer(("127.0.0.1", port), Handler)
    print(server.server_address[1], flush=True)
    server.serve_forever()
    return 0


if __name__ == "__main__":
    sys.exit(main())
