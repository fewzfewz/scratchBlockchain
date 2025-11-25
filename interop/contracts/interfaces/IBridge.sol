// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/**
 * @title IBridge
 * @notice Interface for the cross-chain bridge contract
 */
interface IBridge {
    /**
     * @notice Bridge message structure
     */
    struct BridgeMessage {
        uint64 id;
        uint32 sourceChain;
        uint32 destChain;
        bytes32 sender;
        bytes32 recipient;
        bytes32 token;
        uint256 amount;
        uint64 nonce;
    }

    /**
     * @notice Lock tokens to send to destination chain
     * @param token Token address (address(0) for ETH)
     * @param amount Amount to lock
     * @param recipient Recipient address on destination chain
     * @return messageId The message ID
     */
    function lockTokens(
        address token,
        uint256 amount,
        bytes32 recipient
    ) external payable returns (uint64 messageId);

    /**
     * @notice Unlock tokens received from destination chain
     * @param message Bridge message
     * @param signatures Relayer signatures
     */
    function unlockTokens(
        BridgeMessage calldata message,
        bytes[] calldata signatures
    ) external;
}
