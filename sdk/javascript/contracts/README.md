# Smart Contract Templates

Ready-to-use Solidity smart contract templates for the Modular Blockchain.

## Contracts

| Contract | Description | Standards |
|----------|-------------|-----------|
| `ERC20.sol` | Fungible token with mint, burn, approve, transferFrom | ERC-20 |
| `ERC721.sol` | Non-fungible token with safe transfers, approvals | ERC-721 |
| `DAO.sol` | On-chain governance with proposal/vote/execute flow | Custom |

## Usage

```solidity
import "./ERC20.sol";

contract MyToken is ERC20 {
    constructor() ERC20("MyToken", "MTK", 1_000_000e18) {}
}
```

## Deploying

Use the CLI to deploy:
```bash
npx @modular-blockchain/sdk-cli deploy ERC20 --args "MyToken,MTK,1000000"
```

Or via the SDK:
```ts
import { Wallet, HttpProvider } from "@modular-blockchain/sdk";
import { deployContract } from "@modular-blockchain/sdk/cli";

const provider = new HttpProvider("http://localhost:8545");
const wallet = Wallet.fromPrivateKey("0x...");
const token = await deployContract("ERC20", ["MyToken", "MTK", 1000000], wallet, provider);
```
