// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract ERC1155 {
    string public uri;
    mapping(uint256 => mapping(address => uint256)) public balanceOf;
    mapping(address => mapping(address => bool)) public isApprovedForAll;

    event TransferSingle(address indexed operator, address indexed from, address indexed to, uint256 id, uint256 value);
    event TransferBatch(address indexed operator, address indexed from, address indexed to, uint256[] ids, uint256[] values);
    event ApprovalForAll(address indexed owner, address indexed operator, bool approved);
    event URI(string value, uint256 indexed id);

    constructor(string memory _uri) {
        uri = _uri;
    }

    function setApprovalForAll(address operator, bool approved) external {
        isApprovedForAll[msg.sender][operator] = approved;
        emit ApprovalForAll(msg.sender, operator, approved);
    }

    function safeTransferFrom(address from, address to, uint256 id, uint256 amount, bytes calldata data) external {
        require(to != address(0), "ERC1155: transfer to zero");
        require(from == msg.sender || isApprovedForAll[from][msg.sender], "ERC1155: not approved");

        uint256 fromBalance = balanceOf[id][from];
        require(fromBalance >= amount, "ERC1155: insufficient balance");

        balanceOf[id][from] = fromBalance - amount;
        balanceOf[id][to] += amount;

        emit TransferSingle(msg.sender, from, to, id, amount);
        _doSafeTransferAcceptanceCheck(msg.sender, from, to, id, amount, data);
    }

    function safeBatchTransferFrom(address from, address to, uint256[] calldata ids, uint256[] calldata amounts, bytes calldata data) external {
        require(to != address(0), "ERC1155: transfer to zero");
        require(from == msg.sender || isApprovedForAll[from][msg.sender], "ERC1155: not approved");
        require(ids.length == amounts.length, "ERC1155: length mismatch");

        for (uint256 i = 0; i < ids.length; i++) {
            uint256 id = ids[i];
            uint256 amount = amounts[i];
            uint256 fromBalance = balanceOf[id][from];
            require(fromBalance >= amount, "ERC1155: insufficient balance");
            balanceOf[id][from] = fromBalance - amount;
            balanceOf[id][to] += amount;
        }

        emit TransferBatch(msg.sender, from, to, ids, amounts);
        _doSafeBatchTransferAcceptanceCheck(msg.sender, from, to, ids, amounts, data);
    }

    function balanceOfBatch(address[] calldata owners, uint256[] calldata ids) external view returns (uint256[] memory) {
        require(owners.length == ids.length, "ERC1155: length mismatch");
        uint256[] memory balances = new uint256[](owners.length);
        for (uint256 i = 0; i < owners.length; i++) {
            balances[i] = balanceOf[ids[i]][owners[i]];
        }
        return balances;
    }

    function mint(address to, uint256 id, uint256 amount, bytes calldata data) external {
        balanceOf[id][to] += amount;
        emit TransferSingle(msg.sender, address(0), to, id, amount);
        _doSafeTransferAcceptanceCheck(msg.sender, address(0), to, id, amount, data);
    }

    function mintBatch(address to, uint256[] calldata ids, uint256[] calldata amounts, bytes calldata data) external {
        require(ids.length == amounts.length, "ERC1155: length mismatch");
        for (uint256 i = 0; i < ids.length; i++) {
            balanceOf[ids[i]][to] += amounts[i];
        }
        emit TransferBatch(msg.sender, address(0), to, ids, amounts);
        _doSafeBatchTransferAcceptanceCheck(msg.sender, address(0), to, ids, amounts, data);
    }

    function burn(address from, uint256 id, uint256 amount) external {
        require(from == msg.sender || isApprovedForAll[from][msg.sender], "ERC1155: not approved");
        uint256 fromBalance = balanceOf[id][from];
        require(fromBalance >= amount, "ERC1155: insufficient balance");
        balanceOf[id][from] = fromBalance - amount;
        emit TransferSingle(msg.sender, from, address(0), id, amount);
    }

    function _doSafeTransferAcceptanceCheck(address operator, address from, address to, uint256 id, uint256 amount, bytes memory data) internal {
        if (to.code.length > 0) {
            (bool success, bytes memory result) = to.staticcall(
                abi.encodeWithSignature("onERC1155Received(address,address,uint256,uint256,bytes)", operator, from, id, amount, data)
            );
            require(success && result.length == 32 && abi.decode(result, (bytes4)) == 0xf23a6e61, "ERC1155: rejected");
        }
    }

    function _doSafeBatchTransferAcceptanceCheck(address operator, address from, address to, uint256[] memory ids, uint256[] memory amounts, bytes memory data) internal {
        if (to.code.length > 0) {
            (bool success, bytes memory result) = to.staticcall(
                abi.encodeWithSignature("onERC1155BatchReceived(address,address,uint256[],uint256[],bytes)", operator, from, ids, amounts, data)
            );
            require(success && result.length == 32 && abi.decode(result, (bytes4)) == 0xbc197c81, "ERC1155: rejected");
        }
    }

    function _setURI(string memory newUri) internal {
        uri = newUri;
    }
}
