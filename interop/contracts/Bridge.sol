// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/security/Pausable.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "./interfaces/IBridge.sol";
import "./libraries/MessageLib.sol";

/**
 * @title Bridge
 * @notice Cross-chain bridge for transferring tokens between Ethereum and the modular blockchain
 * @dev Implements lock/unlock mechanism with multi-signature verification
 */
contract Bridge is IBridge, Pausable, ReentrancyGuard, Ownable {
    using SafeERC20 for IERC20;
    using MessageLib for BridgeMessage;

    // State variables
    mapping(address => mapping(address => uint256)) public lockedTokens; // token => user => amount
    mapping(uint64 => bool) public processedMessages; // messageId => processed
    mapping(address => bool) public relayers; // relayer => authorized
    address[] public relayerList;
    uint256 public requiredSignatures;
    uint64 public nextMessageId;
    uint32 public immutable chainId;
    uint32 public immutable destChainId;

    // Constants
    uint256 public constant MIN_SIGNATURES = 2;
    uint256 public constant MAX_RELAYERS = 20;

    // Events
    event TokensLocked(
        uint64 indexed messageId,
        address indexed user,
        address indexed token,
        uint256 amount,
        bytes32 recipient
    );
    
    event TokensUnlocked(
        uint64 indexed messageId,
        address indexed user,
        address indexed token,
        uint256 amount
    );
    
    event MessageProcessed(uint64 indexed messageId);
    event RelayerAdded(address indexed relayer);
    event RelayerRemoved(address indexed relayer);
    event RequiredSignaturesUpdated(uint256 newRequired);

    /**
     * @notice Constructor
     * @param _chainId Ethereum chain ID
     * @param _destChainId Destination chain ID
     * @param _relayers Initial relayer addresses
     * @param _requiredSignatures Number of signatures required
     */
    constructor(
        uint32 _chainId,
        uint32 _destChainId,
        address[] memory _relayers,
        uint256 _requiredSignatures
    ) {
        require(_relayers.length >= MIN_SIGNATURES, "Insufficient relayers");
        require(_requiredSignatures >= MIN_SIGNATURES, "Insufficient required signatures");
        require(_requiredSignatures <= _relayers.length, "Required > available");

        chainId = _chainId;
        destChainId = _destChainId;
        requiredSignatures = _requiredSignatures;
        nextMessageId = 1;

        for (uint256 i = 0; i < _relayers.length; i++) {
            require(_relayers[i] != address(0), "Invalid relayer");
            require(!relayers[_relayers[i]], "Duplicate relayer");
            relayers[_relayers[i]] = true;
            relayerList.push(_relayers[i]);
        }
    }

    /**
     * @notice Lock tokens to send to destination chain
     * @param token Token address (address(0) for ETH)
     * @param amount Amount to lock
     * @param recipient Recipient address on destination chain
     */
    function lockTokens(
        address token,
        uint256 amount,
        bytes32 recipient
    ) external payable override whenNotPaused nonReentrant returns (uint64) {
        require(amount > 0, "Amount must be > 0");
        require(recipient != bytes32(0), "Invalid recipient");

        uint64 messageId = nextMessageId++;

        if (token == address(0)) {
            // Native ETH
            require(msg.value == amount, "Incorrect ETH amount");
        } else {
            // ERC20 token
            require(msg.value == 0, "ETH not accepted for ERC20");
            IERC20(token).safeTransferFrom(msg.sender, address(this), amount);
        }

        lockedTokens[token][msg.sender] += amount;

        emit TokensLocked(messageId, msg.sender, token, amount, recipient);

        return messageId;
    }

    /**
     * @notice Unlock tokens received from destination chain
     * @param message Bridge message
     * @param signatures Relayer signatures
     */
    function unlockTokens(
        BridgeMessage calldata message,
        bytes[] calldata signatures
    ) external override whenNotPaused nonReentrant {
        require(!processedMessages[message.id], "Already processed");
        require(message.sourceChain == destChainId, "Invalid source chain");
        require(message.destChain == chainId, "Invalid dest chain");
        require(message.amount > 0, "Invalid amount");
        require(signatures.length >= requiredSignatures, "Insufficient signatures");

        // Verify signatures
        bytes32 messageHash = message.hash();
        require(verifySignatures(messageHash, signatures), "Invalid signatures");

        // Mark as processed
        processedMessages[message.id] = true;

        // Unlock tokens
        address recipient = address(uint160(uint256(message.recipient)));
        
        if (message.token == bytes32(0)) {
            // Native ETH
            (bool success, ) = recipient.call{value: message.amount}("");
            require(success, "ETH transfer failed");
        } else {
            // ERC20 token
            address token = address(uint160(uint256(message.token)));
            IERC20(token).safeTransfer(recipient, message.amount);
        }

        emit TokensUnlocked(message.id, recipient, address(uint160(uint256(message.token))), message.amount);
        emit MessageProcessed(message.id);
    }

    /**
     * @notice Verify relayer signatures
     * @param messageHash Hash of the message
     * @param signatures Array of signatures
     */
    function verifySignatures(
        bytes32 messageHash,
        bytes[] calldata signatures
    ) internal view returns (bool) {
        bytes32 ethSignedMessageHash = MessageLib.toEthSignedMessageHash(messageHash);
        address[] memory signers = new address[](signatures.length);

        for (uint256 i = 0; i < signatures.length; i++) {
            address signer = MessageLib.recoverSigner(ethSignedMessageHash, signatures[i]);
            
            // Check if signer is authorized relayer
            if (!relayers[signer]) {
                return false;
            }

            // Check for duplicate signers
            for (uint256 j = 0; j < i; j++) {
                if (signers[j] == signer) {
                    return false;
                }
            }
            
            signers[i] = signer;
        }

        return true;
    }

    /**
     * @notice Add a new relayer
     * @param relayer Address of the relayer
     */
    function addRelayer(address relayer) external onlyOwner {
        require(relayer != address(0), "Invalid relayer");
        require(!relayers[relayer], "Already a relayer");
        require(relayerList.length < MAX_RELAYERS, "Max relayers reached");

        relayers[relayer] = true;
        relayerList.push(relayer);

        emit RelayerAdded(relayer);
    }

    /**
     * @notice Remove a relayer
     * @param relayer Address of the relayer
     */
    function removeRelayer(address relayer) external onlyOwner {
        require(relayers[relayer], "Not a relayer");
        require(relayerList.length > requiredSignatures, "Cannot remove, would break threshold");

        relayers[relayer] = false;

        // Remove from list
        for (uint256 i = 0; i < relayerList.length; i++) {
            if (relayerList[i] == relayer) {
                relayerList[i] = relayerList[relayerList.length - 1];
                relayerList.pop();
                break;
            }
        }

        emit RelayerRemoved(relayer);
    }

    /**
     * @notice Update required signatures
     * @param newRequired New required signature count
     */
    function updateRequiredSignatures(uint256 newRequired) external onlyOwner {
        require(newRequired >= MIN_SIGNATURES, "Below minimum");
        require(newRequired <= relayerList.length, "Exceeds relayer count");

        requiredSignatures = newRequired;

        emit RequiredSignaturesUpdated(newRequired);
    }

    /**
     * @notice Pause the bridge
     */
    function pause() external onlyOwner {
        _pause();
    }

    /**
     * @notice Unpause the bridge
     */
    function unpause() external onlyOwner {
        _unpause();
    }

    /**
     * @notice Get locked balance for a user
     * @param token Token address
     * @param user User address
     */
    function getLockedBalance(address token, address user) external view returns (uint256) {
        return lockedTokens[token][user];
    }

    /**
     * @notice Check if message has been processed
     * @param messageId Message ID
     */
    function isProcessed(uint64 messageId) external view returns (bool) {
        return processedMessages[messageId];
    }

    /**
     * @notice Get all relayers
     */
    function getRelayers() external view returns (address[] memory) {
        return relayerList;
    }

    /**
     * @notice Receive ETH
     */
    receive() external payable {}
}
