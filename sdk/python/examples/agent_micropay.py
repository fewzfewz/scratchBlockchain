"""Example: AI Agent micro-payment & staking status check on Nebula.

Run after installing the published package:
    pip install nebula-chain-sdk
"""

from __future__ import annotations

import os
from nebula import NebulaClient

RPC_URL = os.environ.get("NEBULA_RPC_URL", "https://scratchblockchain.onrender.com")


def main() -> None:
    client = NebulaClient(RPC_URL)
    status = client.status()
    validators = client.validators()

    print(f"Connected to Nebula @ {RPC_URL}")
    print(f"Chain height: {status.get('height')} | Finalized: {status.get('finalized_height')}")
    val_list = validators if isinstance(validators, list) else validators.get("validators", [])
    print(f"Active validators: {len(val_list)}")

    try:
        from nebula.wallet import Wallet
    except ImportError:
        print("Tip: install 'nebula-chain-sdk[signing]' for local Ed25519 wallet generation & tx signing.")
        return

    agent_wallet = Wallet.generate()
    balance = client.balance(agent_wallet.address)
    print(f"Agent wallet: {agent_wallet.address} | Balance: {balance}")

    # Sign a deterministic micro-payment transaction (ready for /submit_tx once funded via /faucet)
    tx_hash = Wallet.hash_tx(
        {
            "from": agent_wallet.address,
            "to": "0x000000000000000000000000000000000000dEaD",
            "value": "1000000000000000",  # 0.001 NBL
            "nonce": 0,
        }
    )
    print(f"Prepared micro-payment digest: 0x{tx_hash.hex()}")


if __name__ == "__main__":
    main()
