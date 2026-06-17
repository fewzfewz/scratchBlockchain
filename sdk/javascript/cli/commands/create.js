const fs = require("fs");
const path = require("path");

const TEMPLATES = {
  "erc20": {
    name: "ERC20",
    description: "Fungible token contract",
    template: `// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract {{NAME}} is ERC20 {
    constructor() ERC20("{{NAME}}", "{{SYMBOL}}") {
        _mint(msg.sender, 1_000_000 * 10 ** decimals());
    }
}
`,
  },
  "erc721": {
    name: "ERC721",
    description: "Non-fungible token contract",
    template: `// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC721/ERC721.sol";

contract {{NAME}} is ERC721 {
    uint256 private _nextTokenId;

    constructor() ERC721("{{NAME}}", "{{SYMBOL}}") {}

    function safeMint(address to) public {
        uint256 tokenId = _nextTokenId++;
        _safeMint(to, tokenId);
    }
}
`,
  },
  "dao": {
    name: "DAO",
    description: "Governance DAO contract",
    template: `// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract {{NAME}} {
    struct Proposal {
        uint256 id;
        address proposer;
        string description;
        uint256 forVotes;
        uint256 againstVotes;
        uint256 endBlock;
        bool executed;
        bytes data;
    }

    uint256 public proposalCount;
    mapping(uint256 => Proposal) public proposals;

    event ProposalCreated(uint256 id, address proposer, string description);
    event Voted(uint256 proposalId, address voter, bool support);
    event Executed(uint256 proposalId);

    function propose(string calldata description, bytes calldata data) external {
        proposalCount++;
        proposals[proposalCount] = Proposal(proposalCount, msg.sender, description, 0, 0, block.number + 100, false, data);
        emit ProposalCreated(proposalCount, msg.sender, description);
    }

    function vote(uint256 proposalId, bool support) external {
        Proposal storage p = proposals[proposalId];
        if (support) p.forVotes++; else p.againstVotes++;
        emit Voted(proposalId, msg.sender, support);
    }

    function execute(uint256 proposalId) external {
        Proposal storage p = proposals[proposalId];
        require(!p.executed && block.number > p.endBlock && p.forVotes > p.againstVotes);
        p.executed = true;
        (bool ok,) = address(this).call(p.data);
        require(ok);
        emit Executed(proposalId);
    }
}
`,
  },
};

module.exports = async function create(type, options) {
  const lowerType = type.toLowerCase();
  const template = TEMPLATES[lowerType];

  if (!template) {
    console.error(`Unknown type: ${type}`);
    console.log(`Available: ${Object.keys(TEMPLATES).join(", ")}`);
    process.exit(1);
  }

  const name = options.name || template.name;
  const outputDir = path.resolve(options.output || ".");
  const filename = path.join(outputDir, `${name}.sol`);

  if (fs.existsSync(filename)) {
    console.error(`File already exists: ${filename}`);
    process.exit(1);
  }

  const symbol = name.slice(0, 5).toUpperCase();
  let content = template.template
    .replace(/{{NAME}}/g, name)
    .replace(/{{SYMBOL}}/g, symbol);

  fs.writeFileSync(filename, content);
  console.log(`Created ${lowerType} contract: ${filename}`);
};
