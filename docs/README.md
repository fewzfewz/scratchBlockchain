# Nebula Documentation

Technical documentation for the Scratch Blockchain (Nebula). Covers architecture, API reference, and onboarding guides.

**Also available in-app:** `http://localhost:5173/docs` and interactive Swagger at `/api-docs`.

## Contents

| Path | Description |
|------|-------------|
| `openapi.yaml` | OpenAPI 3.0 spec (core endpoints; new routes listed in root README) |
| `testnet-onboarding.md` | Public testnet participation guide |
| `validator-onboarding.md` | Validator setup guide |

## RPC Reference (August 2026)

The node exposes **28 endpoints** (27 HTTP + WebSocket). See [README.md](../README.md#rpc-api) for the full table.

New routes include:
- Account abstraction: `POST /submit_user_operation`
- MEV: `POST /mev/commit`, `/mev/reveal`, `/mev/encrypted`, `/mev/decryption_share`
- Staking: `POST /delegate`, `POST /validators/register`
- Slashing: `GET /slashing/events`
- Real-time: `GET /ws` (WebSocket)

## Quick Start

Serve static docs (optional):

```bash
python3 -m http.server 8084
```

For live API testing, use the running node at `http://localhost:8545` or the frontend Swagger UI.
