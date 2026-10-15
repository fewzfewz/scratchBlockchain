// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

/**
 * @title NebulaDemoToken
 * @notice ERC20 deployed from the web UI's "Deploy Contract" page. The
 *         deployer receives 1,000,000 tokens; anyone may mint more (demo only).
 */
contract NebulaDemoToken is ERC20 {
    constructor() ERC20("Nebula Demo Token", "NDT") {
        _mint(msg.sender, 1_000_000 * 10 ** decimals());
    }

    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}
