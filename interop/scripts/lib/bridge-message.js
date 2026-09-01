/**
 * Bridge message hashing and signing (matches MessageLib.sol).
 */
const { ethers } = require('ethers');

function hashBridgeMessage(msg) {
  const packed = ethers.solidityPacked(
    ['uint64', 'uint32', 'uint32', 'bytes32', 'bytes32', 'bytes32', 'uint256', 'uint64'],
    [
      msg.id,
      msg.sourceChain,
      msg.destChain,
      msg.sender,
      msg.recipient,
      msg.token,
      msg.amount,
      msg.nonce,
    ],
  );
  return ethers.keccak256(packed);
}

function addrToBytes32(addr) {
  return ethers.zeroPadValue(ethers.getAddress(addr), 32);
}

function buildMessageFromPending(pending) {
  const amount = BigInt(pending.amount);
  return {
    id: BigInt(pending.message_id),
    sourceChain: pending.source_chain,
    destChain: pending.dest_chain,
    sender: addrToBytes32(pending.sender),
    recipient: addrToBytes32(pending.eth_recipient),
    token: ethers.ZeroHash,
    amount,
    nonce: BigInt(pending.nonce),
  };
}

async function signBridgeMessage(msg, wallet) {
  const hash = hashBridgeMessage(msg);
  return wallet.signMessage(ethers.getBytes(hash));
}

module.exports = {
  hashBridgeMessage,
  addrToBytes32,
  buildMessageFromPending,
  signBridgeMessage,
};
