# Nebula Faucet

Test token faucet for the Scratch Blockchain. Dispenses NBL tokens to local wallet addresses.

**Tech Stack:** Rust (warp HTTP server), vanilla HTML/CSS/JS frontend.

**Frontend** (`index.html`) submits requests to `POST /faucet` on the faucet backend service.
- 100 NBL per request
- 24-hour cooldown (client-side via localStorage)
- Address validation (0x + 40 hex chars)

## Quick Start

Start the faucet backend (token dispenser):
```bash
cargo run --release --bin node faucet
```
The warp server listens on `http://localhost:3006/faucet`.

Serve the frontend separately:
```bash
python3 -m http.server 8082 --directory faucet/
```

Or use the all-in-one launcher:
```bash
./scripts/start-frontends.sh
```
