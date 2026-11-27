const { expect } = require("chai");
const { ethers } = require("hardhat");

describe("Bridge", function () {
  const ETH_CHAIN_ID = 11155111;
  const NEBULA_CHAIN_ID = 1;

  async function deployBridgeFixture() {
    const [owner, relayer1, relayer2, relayer3, user, outsider] = await ethers.getSigners();
    const Bridge = await ethers.getContractFactory("Bridge");
    const bridge = await Bridge.deploy(
      ETH_CHAIN_ID,
      NEBULA_CHAIN_ID,
      [relayer1.address, relayer2.address],
      2
    );
    await bridge.waitForDeployment();
    return { bridge, owner, relayer1, relayer2, relayer3, user, outsider };
  }

  function hashBridgeMessage(msg) {
    return ethers.solidityPackedKeccak256(
      ["uint64", "uint32", "uint32", "bytes32", "bytes32", "bytes32", "uint256", "uint64"],
      [
        msg.id,
        msg.sourceChain,
        msg.destChain,
        msg.sender,
        msg.recipient,
        msg.token,
        msg.amount,
        msg.nonce,
      ]
    );
  }

  it("rejects insufficient or duplicate relayers in constructor", async function () {
    const [, relayer1] = await ethers.getSigners();
    const Bridge = await ethers.getContractFactory("Bridge");

    await expect(
      Bridge.deploy(ETH_CHAIN_ID, NEBULA_CHAIN_ID, [relayer1.address], 2)
    ).to.be.revertedWith("Insufficient relayers");

    await expect(
      Bridge.deploy(ETH_CHAIN_ID, NEBULA_CHAIN_ID, [relayer1.address, relayer1.address], 2)
    ).to.be.revertedWith("Duplicate relayer");
  });

  it("locks native ETH and emits TokensLocked", async function () {
    const { bridge, user } = await deployBridgeFixture();
    const amount = ethers.parseEther("1.5");
    const recipient = ethers.zeroPadValue(user.address, 32);

    await expect(
      bridge.connect(user).lockTokens(ethers.ZeroAddress, amount, recipient, { value: amount })
    )
      .to.emit(bridge, "TokensLocked")
      .withArgs(1n, user.address, ethers.ZeroAddress, amount, recipient);

    expect(await bridge.lockedTokens(ethers.ZeroAddress, user.address)).to.equal(amount);
  });

  it("unlocks ETH with 2 valid relayer signatures and prevents replay", async function () {
    const { bridge, relayer1, relayer2, user } = await deployBridgeFixture();
    const amount = ethers.parseEther("1.0");
    const recipientBytes32 = ethers.zeroPadValue(user.address, 32);

    await bridge.connect(user).lockTokens(ethers.ZeroAddress, amount, recipientBytes32, { value: amount });

    const message = {
      id: 1n,
      sourceChain: NEBULA_CHAIN_ID,
      destChain: ETH_CHAIN_ID,
      sender: recipientBytes32,
      recipient: recipientBytes32,
      token: ethers.ZeroHash,
      amount,
      nonce: 0n,
    };

    const msgHash = hashBridgeMessage(message);
    const sig1 = await relayer1.signMessage(ethers.getBytes(msgHash));
    const sig2 = await relayer2.signMessage(ethers.getBytes(msgHash));

    await expect(bridge.unlockTokens(message, [sig1, sig2]))
      .to.emit(bridge, "TokensUnlocked")
      .withArgs(1n, user.address, ethers.ZeroAddress, amount);

    expect(await bridge.processedMessages(1n)).to.equal(true);

    await expect(bridge.unlockTokens(message, [sig1, sig2])).to.be.revertedWith("Already processed");
  });

  it("rejects duplicate or unauthorized relayer signatures", async function () {
    const { bridge, relayer1, outsider, user } = await deployBridgeFixture();
    const amount = ethers.parseEther("0.5");
    const recipientBytes32 = ethers.zeroPadValue(user.address, 32);

    await bridge.connect(user).lockTokens(ethers.ZeroAddress, amount, recipientBytes32, { value: amount });

    const message = {
      id: 2n,
      sourceChain: NEBULA_CHAIN_ID,
      destChain: ETH_CHAIN_ID,
      sender: recipientBytes32,
      recipient: recipientBytes32,
      token: ethers.ZeroHash,
      amount,
      nonce: 1n,
    };

    const msgHash = hashBridgeMessage(message);
    const sig1 = await relayer1.signMessage(ethers.getBytes(msgHash));
    const sigOutsider = await outsider.signMessage(ethers.getBytes(msgHash));

    await expect(bridge.unlockTokens(message, [sig1, sig1])).to.be.revertedWith("Invalid signatures");
    await expect(bridge.unlockTokens(message, [sig1, sigOutsider])).to.be.revertedWith("Invalid signatures");
  });

  it("supports owner pause/unpause and relayer management", async function () {
    const { bridge, relayer3, user } = await deployBridgeFixture();
    const amount = ethers.parseEther("0.1");
    const recipient = ethers.zeroPadValue(user.address, 32);

    await bridge.pause();
    await expect(
      bridge.connect(user).lockTokens(ethers.ZeroAddress, amount, recipient, { value: amount })
    ).to.be.reverted;

    await bridge.unpause();
    await expect(
      bridge.connect(user).lockTokens(ethers.ZeroAddress, amount, recipient, { value: amount })
    ).to.emit(bridge, "TokensLocked");

    await expect(bridge.addRelayer(relayer3.address))
      .to.emit(bridge, "RelayerAdded")
      .withArgs(relayer3.address);
    expect(await bridge.relayers(relayer3.address)).to.equal(true);
  });
});
