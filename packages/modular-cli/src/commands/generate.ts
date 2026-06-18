import * as fs from "node:fs";
import * as path from "node:path";
import chalk from "chalk";

const CONTRACT_TEMPLATES = {
  erc20: `// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract {{NAME}} {
    string public name = "{{NAME}}";
    string public symbol = "{{SYMBOL}}";
    uint8 public decimals = 18;
    uint256 public totalSupply;

    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;

    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);

    constructor(uint256 _initialSupply) {
        _mint(msg.sender, _initialSupply);
    }

    function transfer(address to, uint256 amount) external returns (bool) {
        require(balanceOf[msg.sender] >= amount, "insufficient balance");
        balanceOf[msg.sender] -= amount;
        balanceOf[to] += amount;
        emit Transfer(msg.sender, to, amount);
        return true;
    }

    function approve(address spender, uint256 amount) external returns (bool) {
        allowance[msg.sender][spender] = amount;
        emit Approval(msg.sender, spender, amount);
        return true;
    }

    function transferFrom(address from, address to, uint256 amount) external returns (bool) {
        require(allowance[from][msg.sender] >= amount, "insufficient allowance");
        allowance[from][msg.sender] -= amount;
        require(balanceOf[from] >= amount, "insufficient balance");
        balanceOf[from] -= amount;
        balanceOf[to] += amount;
        emit Transfer(from, to, amount);
        return true;
    }

    function _mint(address to, uint256 amount) internal {
        balanceOf[to] += amount;
        totalSupply += amount;
        emit Transfer(address(0), to, amount);
    }
}
`,

  erc721: `// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract {{NAME}} {
    string public name = "{{NAME}}";
    string public symbol = "{{SYMBOL}}";

    mapping(uint256 => address) private _owners;
    mapping(address => uint256) private _balances;
    mapping(uint256 => address) private _tokenApprovals;
    mapping(address => mapping(address => bool)) private _operatorApprovals;

    event Transfer(address indexed from, address indexed to, uint256 indexed tokenId);
    event Approval(address indexed owner, address indexed approved, uint256 indexed tokenId);
    event ApprovalForAll(address indexed owner, address indexed operator, bool approved);

    constructor() {}

    function ownerOf(uint256 tokenId) public view returns (address) {
        address owner = _owners[tokenId];
        require(owner != address(0), "invalid token ID");
        return owner;
    }

    function balanceOf(address owner) public view returns (uint256) {
        require(owner != address(0), "zero address");
        return _balances[owner];
    }

    function mint(address to, uint256 tokenId) external {
        require(to != address(0), "mint to zero");
        require(_owners[tokenId] == address(0), "already minted");
        _balances[to]++;
        _owners[tokenId] = to;
        emit Transfer(address(0), to, tokenId);
    }

    function transferFrom(address from, address to, uint256 tokenId) external {
        require(_isApprovedOrOwner(msg.sender, tokenId), "not approved");
        address owner = _owners[tokenId];
        require(owner == from, "wrong owner");
        require(to != address(0), "transfer to zero");
        delete _tokenApprovals[tokenId];
        _balances[from]--;
        _balances[to]++;
        _owners[tokenId] = to;
        emit Transfer(from, to, tokenId);
    }

    function approve(address to, uint256 tokenId) external {
        address owner = _owners[tokenId];
        require(msg.sender == owner || isApprovedForAll(owner, msg.sender), "not approved");
        _tokenApprovals[tokenId] = to;
        emit Approval(owner, to, tokenId);
    }

    function getApproved(uint256 tokenId) public view returns (address) {
        require(_owners[tokenId] != address(0), "invalid token ID");
        return _tokenApprovals[tokenId];
    }

    function setApprovalForAll(address operator, bool approved) external {
        _operatorApprovals[msg.sender][operator] = approved;
        emit ApprovalForAll(msg.sender, operator, approved);
    }

    function isApprovedForAll(address owner, address operator) public view returns (bool) {
        return _operatorApprovals[owner][operator];
    }

    function _isApprovedOrOwner(address spender, uint256 tokenId) internal view returns (bool) {
        address owner = _owners[tokenId];
        return spender == owner || _tokenApprovals[tokenId] == spender || isApprovedForAll(owner, spender);
    }
}
`,
};

export async function generateCommand(type: string, options: { name?: string; template?: string }) {
  if (type !== "contract") {
    console.error(chalk.red(`Generation for "${type}" not yet supported. Use: contract`));
    process.exit(1);
  }

  const template = options.template || "erc20";
  const name = options.name || "MyContract";

  const tmpl = CONTRACT_TEMPLATES[template];
  if (!tmpl) {
    console.error(chalk.red(`Unknown template "${template}". Available: ${Object.keys(CONTRACT_TEMPLATES).join(", ")}`));
    process.exit(1);
  }

  const filename = `${name}.sol`;
  let content = tmpl
    .replace(/\{\{NAME\}\}/g, name)
    .replace(/\{\{SYMBOL\}\}/g, name.slice(0, 6).toUpperCase());

  const filepath = path.resolve(process.cwd(), filename);
  fs.writeFileSync(filepath, content);

  console.log(chalk.green(`\n  ✓ Generated ${filename}\n`));
  console.log(chalk.dim(`  Template: ${template}`));
  console.log(chalk.dim(`  Path: ${filepath}\n`));
  console.log(chalk.dim(`  Next: compile with solc or deploy via "modular deploy"\n`));
}
