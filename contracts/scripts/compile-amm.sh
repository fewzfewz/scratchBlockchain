#!/usr/bin/env bash
# Compile SimpleAMM from repo contracts/ and export init bytecode for deploy scripts.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="$ROOT/contracts/bytecode"
mkdir -p "$OUT"

cp "$ROOT/contracts/SimpleAMM.sol" "$ROOT/interop/contracts/SimpleAMM.sol"

cd "$ROOT/interop"
npm install --silent 2>/dev/null || true
npx hardhat compile

node <<'NODE'
const fs = require('fs');
const path = require('path');
const artifact = JSON.parse(
  fs.readFileSync(
    path.join('artifacts/contracts/SimpleAMM.sol/SimpleAMM.json'),
    'utf8',
  ),
);
const outDir = path.join('..', 'contracts', 'bytecode');
fs.mkdirSync(outDir, { recursive: true });
const payload = {
  contract: 'SimpleAMM',
  bytecode: artifact.bytecode,
  deployedBytecode: artifact.deployedBytecode,
  abi: artifact.abi,
  compiledAt: new Date().toISOString(),
};
fs.writeFileSync(path.join(outDir, 'SimpleAMM.json'), JSON.stringify(payload, null, 2));
console.log('Wrote contracts/bytecode/SimpleAMM.json (' + artifact.bytecode.length + ' chars)');
NODE
