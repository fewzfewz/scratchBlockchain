# Local Deployment Assets

This directory contains deployment-specific assets for the local testnet environment.

**Files:**
- `faucet.html` — standalone faucet page for the Modular Blockchain Testnet. Self-contained (all CSS/JS inline). Dispenses 1000 NBL per request via `POST http://localhost:3001/faucet`. Includes 24h cooldown and request statistics via localStorage.
