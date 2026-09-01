# Nebula Python SDK

Minimal stdlib-only client for the Nebula node RPC.

```bash
pip install -e sdk/python
```

```python
from nebula import NebulaClient

client = NebulaClient("http://localhost:8545")
print(client.status())
print(client.governance())
```

Governance writes still require a signed `POST /submit_tx` payload (use the JavaScript SDK or `tests/localhost/scripts/lib/gov-tx.js` for signing).
