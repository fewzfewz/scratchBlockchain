# Nebula Python SDK (`nebula-chain-sdk`)

Minimal stdlib-only client for the Nebula modular blockchain RPC.

## Install

```bash
pip install nebula-chain-sdk
```

With optional Ed25519 signing support:

```bash
pip install "nebula-chain-sdk[signing]"
```

## Quick Start (Live Network)

```python
from nebula import NebulaClient
from nebula.wallet import Wallet

client = NebulaClient("https://scratchblockchain.onrender.com")
print(client.status())
print(client.validators())

wallet = Wallet.generate()
print("Address:", wallet.address)
```

See [examples/agent_micropay.py](examples/agent_micropay.py) for an AI-agent micro-payment & validator status example.
