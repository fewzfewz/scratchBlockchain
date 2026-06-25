# Nebula Documentation

Technical documentation site for the Scratch Blockchain (Nebula). Covers architecture, API reference, and onboarding guides.

**Tech Stack:** Vanilla HTML, CSS, JS. Highlight.js for code syntax highlighting.

**Sections:**
- **Getting Started** — introduction, installation, quick start
- **Architecture** — consensus (GRANDPA-style finality), networking (libp2p), storage, Ed25519 signatures, parallel execution
- **API Reference** — all RPC endpoints and data types

Also includes markdown guides: `testnet-onboarding.md`, `validator-onboarding.md`.

## Quick Start

Serve with any HTTP server:

```bash
python3 -m http.server 8084
```
