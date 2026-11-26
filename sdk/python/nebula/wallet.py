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
        "Install PyNaCl for signing: pip install 'nebula-chain-sdk[signing]'"
    ) from exc

from nebula.client import NebulaClient


TX_DOMAIN = b"NEBULA_TX_V1"


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

    @staticmethod
    def hash_tx(tx: dict[str, Any]) -> bytes:
        """Canonical tx hash; must match Rust ``Transaction::hash``.

        Domain tag, length-prefixed payload, and presence tags on
        ``chain_id`` / ``to`` so no two transactions share an encoding.
        """
        u64 = lambda v: struct.pack("<Q", int(v))  # noqa: E731
        payload = bytes(tx.get("payload", []))
        parts = [
            TX_DOMAIN,
            bytes(tx["sender"][:20]),
            u64(tx["nonce"]),
            bytes(tx["public_key"]),
            u64(len(payload)),
            payload,
            u64(tx["gas_limit"]),
            u64(tx["max_fee_per_gas"]),
            u64(tx["max_priority_fee_per_gas"]),
        ]
        chain_id = tx.get("chain_id")
        parts.append(b"\x01" + u64(chain_id) if chain_id is not None else b"\x00")
        to = tx.get("to")
        parts.append(b"\x01" + bytes(to[:20]) if to else b"\x00")
        parts.append(u64(tx.get("value", 0)))
        return _sha256(b"".join(parts))

    def sign_tx(self, tx: dict[str, Any]) -> dict[str, Any]:
        """Set ``public_key`` and ``signature`` on a node-format tx dict."""
        tx["public_key"] = list(self.public_key)
        tx["signature"] = list(self._signing.sign(self.hash_tx(tx)).signature)
        return tx

    def submit_governance_action(
        self, client: NebulaClient, action: dict[str, Any]
    ) -> Any:
        bal = client.balance(self.address)
        nonce = int(bal.get("nonce", 0))
        payload = list(json.dumps(action).encode("utf-8"))
        sender = bytes.fromhex(self.address.removeprefix("0x"))
        tx = {
            "sender": list(sender),
            "nonce": nonce,
            "value": 0,
            "gas_limit": 21000,
            "max_fee_per_gas": 1_000_000_000,
            "max_priority_fee_per_gas": 1_000_000_000,
            "payload": payload,
            "chain_id": 1,
            "to": None,
            "signature": [],
        }
        return client.submit_tx(self.sign_tx(tx))

    def send_nbl(
        self,
        client: NebulaClient,
        to: str,
        amount_nbl: float | str,
        chain_id: int = 1,
    ) -> Any:
        """Sign and submit a native NBL transfer in one call."""
        from decimal import Decimal

        bal = client.balance(self.address)
        nonce = int(bal.get("nonce", 0))
        gas_info = client.gas_price()
        base_fee = int(gas_info.get("base_fee", 1_000_000_000)) if isinstance(gas_info, dict) else 1_000_000_000
        value_wei = int(Decimal(str(amount_nbl)) * Decimal(10**18))
        sender = bytes.fromhex(self.address.removeprefix("0x"))
        recipient = bytes.fromhex(to.removeprefix("0x"))
        tx = {
            "sender": list(sender),
            "nonce": nonce,
            "value": value_wei,
            "gas_limit": 21_000,
            "max_fee_per_gas": max(base_fee * 2, 1_000_000_000),
            "max_priority_fee_per_gas": 1_000_000_000,
            "payload": [],
            "chain_id": chain_id,
            "to": list(recipient),
            "signature": [],
        }
        return client.submit_tx(self.sign_tx(tx))
