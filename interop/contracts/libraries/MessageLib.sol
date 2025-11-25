// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "../interfaces/IBridge.sol";

/**
 * @title MessageLib
 * @notice Library for bridge message encoding and signature verification
 */
library MessageLib {
    /**
     * @notice Hash a bridge message
     * @param message The bridge message
     * @return Hash of the message
     */
    function hash(IBridge.BridgeMessage calldata message) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked(
            message.id,
            message.sourceChain,
            message.destChain,
            message.sender,
            message.recipient,
            message.token,
            message.amount,
            message.nonce
        ));
    }

    /**
     * @notice Convert hash to Ethereum signed message hash
     * @param messageHash Original message hash
     * @return Ethereum signed message hash
     */
    function toEthSignedMessageHash(bytes32 messageHash) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked("\x19Ethereum Signed Message:\n32", messageHash));
    }

    /**
     * @notice Recover signer from signature
     * @param ethSignedMessageHash Ethereum signed message hash
     * @param signature Signature bytes
     * @return Signer address
     */
    function recoverSigner(
        bytes32 ethSignedMessageHash,
        bytes calldata signature
    ) internal pure returns (address) {
        require(signature.length == 65, "Invalid signature length");

        bytes32 r;
        bytes32 s;
        uint8 v;

        assembly {
            r := calldataload(signature.offset)
            s := calldataload(add(signature.offset, 32))
            v := byte(0, calldataload(add(signature.offset, 64)))
        }

        // EIP-2 still allows signature malleability for ecrecover(). Remove this possibility
        // and make the signature unique. Appendix F in the Ethereum Yellow paper
        require(uint256(s) <= 0x7FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF5D576E7357A4501DDFE92F46681B20A0, "Invalid signature 's' value");

        address signer = ecrecover(ethSignedMessageHash, v, r, s);
        require(signer != address(0), "Invalid signature");

        return signer;
    }
}
