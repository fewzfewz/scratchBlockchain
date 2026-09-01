"""Ed25519 wallet helpers for signed Nebula transactions (requires PyNaCl)."""

from __future__ import annotations

import hashlib
import json
import struct
from typing import Any

try:
    import nacl.signing
except ImportError as exc:  # pragma: no cover
    raise ImportError(
        "Install PyNaCl for signing: pip install 'nebula-sdk[signing]'"
    ) from exc

from nebula.client import NebulaClient


def _sha256(data: bytes) -> bytes:
    return hashlib.sha256(data).digest()


def derive_address(public_key: bytes) -> str:
    return "0x" + _sha256(public_key)[-20:].hex()


class Wallet:
    def __init__(self, secret_key: bytes):
        self._signing = nacl.signing.SigningKey(secret_key)
        self.public_key = bytes(self._signing.verify_key)
        self.address = derive_address(self.public_key)

    @classmethod
    def generate(cls) -> "Wallet":
        return cls(nacl.signing.SigningKey.generate().encode())

    def _hash_tx(self, tx: dict[str, Any]) -> bytes:
        parts: list[bytes] = []
        sender = bytes(tx["sender"][:20])
        parts.append(sender)
        parts.append(struct.pack("<Q", int(tx["nonce"])))
        parts.append(bytes(tx["payload"]))
        parts.append(struct.pack("<Q", int(tx["gas_limit"])))
        parts.append(struct.pack("<Q", int(tx["max_fee_per_gas"])))
        parts.append(struct.pack("<Q", int(tx["max_priority_fee_per_gas"])))
        if tx.get("chain_id") is not None:
            parts.append(struct.pack("<Q", int(tx["chain_id"])))
        if tx.get("to"):
            parts.append(bytes(tx["to"][:20]))
        parts.append(struct.pack("<Q", int(tx.get("value", 0))))
        return _sha256(b"".join(parts))

    def submit_governance_action(
        self, client: NebulaClient, action: dict[str, Any]
    ) -> Any:
        bal = client.balance(self.address)
        nonce = int(bal.get("nonce", 0))
        body = json.dumps(action).encode("utf-8")
        payload = list(self.public_key) + list(body)
        sender = bytes.fromhex(self.address.removeprefix("0x"))
        tx = {
            "sender": list(sender),
            "nonce": nonce,
            "value": 0,
            "gas_limit": 21000,
            "max_fee_per_gas": 1_000_000_000,
            "max_priority_fee_per_gas": 100_000_000,
            "payload": payload,
            "chain_id": 1,
            "signature": [],
        }
        msg = self._hash_tx(tx)
        sig = self._signing.sign(msg).signature
        tx["signature"] = list(sig)
        return client.submit_tx(tx)
