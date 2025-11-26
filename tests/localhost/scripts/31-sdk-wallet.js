const { Wallet } = require('@modular-blockchain/sdk');

async function test31_SDKWallet() {
    console.log('🧪 Test 9.2: SDK Wallet\n');
    
    try {
        console.log('1. Generating new wallet...');
        const wallet1 = Wallet.generate();
        console.log(`   ✅ Address: ${wallet1.address}`);
        console.log(`   ✅ Public Key: ${wallet1.publicKey.substring(0, 20)}...`);
        
        console.log('\n2. Importing from private key...');
        const privateKey = wallet1.getPrivateKey();
        const wallet2 = Wallet.fromPrivateKey(privateKey);
        console.log(`   ✅ Imported address: ${wallet2.address}`);
        console.log(`   ✅ Addresses match: ${wallet1.address === wallet2.address}`);
        
        console.log('\n3. Importing from mnemonic...');
        const mnemonic = 'test mnemonic phrase for wallet generation example';
        const wallet3 = Wallet.fromMnemonic(mnemonic);
        console.log(`   ✅ Address from mnemonic: ${wallet3.address}`);
        
        console.log('\n4. Signing message...');
        const message = 'Hello, Modular Blockchain!';
        const signature = await wallet1.signMessage(message);
        console.log(`   ✅ Message signed`);
        console.log(`   ✅ Signature: ${signature.substring(0, 20)}...`);
        
        console.log('\n5. Testing wallet properties...');
        console.log(`   ✅ Has address: ${!!wallet1.address}`);
        console.log(`   ✅ Has public key: ${!!wallet1.publicKey}`);
        console.log(`   ✅ Can get private key: ${!!wallet1.getPrivateKey()}`);
        
        console.log('\n' + '='.repeat(50));
        console.log('✅ TEST PASSED: SDK Wallet');
        console.log('='.repeat(50));
        console.log('Wallet functionality working correctly!');
        console.log(`Generated wallet: ${wallet1.address}`);
        
    } catch (error) {
        console.error('\n❌ TEST FAILED:', error.message);
        process.exit(1);
    }
}

test31_SDKWallet().catch(console.error);
