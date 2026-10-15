// Writes the compiled demo contracts' creation bytecode for the web UI.
// Run after `npx hardhat compile`: node scripts/export-demo-contracts.js
const fs = require("fs");
const path = require("path");

const out = path.join(__dirname, "../../frontend/src/lib/demoContracts.json");
const read = (name) =>
  JSON.parse(fs.readFileSync(path.join(__dirname, `../artifacts/contracts/demo/${name}.sol/${name}.json`), "utf8"));

const contracts = {
  ERC20: { label: "ERC20 Token (NebulaDemoToken)", bytecode: read("NebulaDemoToken").bytecode },
  ERC721: { label: "ERC721 NFT (NebulaDemoNFT)", bytecode: read("NebulaDemoNFT").bytecode },
};
fs.writeFileSync(out, JSON.stringify(contracts, null, 2) + "\n");
console.log(`Wrote ${out}`);
