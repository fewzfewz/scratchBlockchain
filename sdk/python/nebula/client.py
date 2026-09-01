"""Minimal HTTP client for Nebula node RPC."""

from __future__ import annotations

import json
import urllib.error
import urllib.request
from typing import Any, Optional


class NebulaClient:
    """Lightweight RPC client (stdlib only — no extra dependencies)."""

    def __init__(self, base_url: str = "http://localhost:8545", timeout: float = 30.0):
        self.base_url = base_url.rstrip("/")
        self.timeout = timeout

    def _request(
        self,
        method: str,
        path: str,
        body: Optional[dict[str, Any]] = None,
    ) -> Any:
        url = f"{self.base_url}{path}"
        data = None
        headers = {"Accept": "application/json"}
        if body is not None:
            data = json.dumps(body).encode("utf-8")
            headers["Content-Type"] = "application/json"
        req = urllib.request.Request(url, data=data, headers=headers, method=method)
        try:
            with urllib.request.urlopen(req, timeout=self.timeout) as resp:
                raw = resp.read().decode("utf-8")
                return json.loads(raw) if raw else None
        except urllib.error.HTTPError as e:
            detail = e.read().decode("utf-8", errors="replace")
            raise RuntimeError(f"HTTP {e.code}: {detail}") from e

    def get(self, path: str) -> Any:
        return self._request("GET", path)

    def post(self, path: str, body: dict[str, Any]) -> Any:
        return self._request("POST", path, body)

    # --- Common reads ---

    def health(self) -> Any:
        return self.get("/health")

    def status(self) -> Any:
        return self.get("/status")

    def governance(self) -> Any:
        return self.get("/governance")

    def validators(self) -> Any:
        return self.get("/validators")

    def balance(self, address: str) -> Any:
        addr = address.removeprefix("0x")
        return self.get(f"/balance/{addr}")

    def runtime_version(self) -> Any:
        return self.get("/runtime/version")

    def list_wasm_contracts(self) -> list[str]:
        data = self.get("/wasm/contracts")
        return data.get("contracts", []) if isinstance(data, dict) else []

    # --- Writes ---

    def submit_tx(self, tx: dict[str, Any]) -> Any:
        return self.post("/submit_tx", tx)

    def deploy_wasm(self, name: str, wasm_base64: str) -> Any:
        return self.post("/deploy_wasm", {"name": name, "wasm": wasm_base64})

    def call_wasm(self, name: str, func: str, arg: int = 0) -> Any:
        return self.post("/call_wasm", {"name": name, "func": func, "arg": arg})

    def submit_user_operation(self, op: dict[str, Any]) -> Any:
        return self.post("/submit_user_operation", op)

    def bridge_mint(self, payload: dict[str, Any]) -> Any:
        return self.post("/bridge/mint", payload)

    def bridge_unlock(self, payload: dict[str, Any]) -> Any:
        return self.post("/bridge/unlock", payload)
