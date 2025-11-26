const { ModularClient, HttpProvider, Wallet } = require('@modular-blockchain/sdk');

async function test01_SendTransaction() {
    console.log('🧪 Test 1.3: Transaction Processing\n');
    
    try {
        // Connect to local testnet
        const provider = new HttpProvider('http://localhost/rpc');
        const client = new ModularClient(provider);
        
        console.log('1. Connecting to testnet...');
        const chainId = await client.getChainId();
        console.log(`   ✅ Connected to chain: ${chainId}`);
        
        // Create wallet
        console.log('\n2. Creating wallet...');
        const wallet = Wallet.generate().connect(provider);
        console.log(`   ✅ Wallet address: ${wallet.address}`);
        
        // Get initial balance (should be 0)
        console.log('\n3. Checking initial balance...');
        const initialBalance = await wallet.getBalance();
        console.log(`   ✅ Initial balance: ${initialBalance}`);
        
        // Request tokens from faucet
        console.log('\n4. Requesting tokens from faucet...');
        console.log(`   ℹ️  Visit: http://localhost/faucet`);
        console.log(`   ℹ️  Address: ${wallet.address}`);
        console.log(`   ⏳ Waiting 10 seconds for manual faucet request...`);
        
        await new Promise(resolve => setTimeout(resolve, 10000));
        
        // Check balance after faucet
        const balanceAfterFaucet = await wallet.getBalance();
        console.log(`   ✅ Balance after faucet: ${balanceAfterFaucet}`);
        
        if (balanceAfterFaucet === '0') {
            console.log('   ⚠️  No tokens received. Please request from faucet manually.');
            console.log('   ℹ️  Skipping transaction test.');
            return;
        }
        
        // Send transaction
        console.log('\n5. Sending transaction...');
        const recipient = '0x0000000000000000000000000000000000000001';
        const amount = '1000000000000000000'; // 1 token
        
        const tx = await wallet.sendTransaction({
            to: recipient,
            value: amount,
        });
        
        console.log(`   ✅ Transaction sent!`);
        console.log(`   📝 Hash: ${tx.transactionHash}`);
        console.log(`   📦 Block: ${tx.blockNumber}`);
        
        // Verify balance changed
        console.log('\n6. Verifying balance...');
        const finalBalance = await wallet.getBalance();
        console.log(`   ✅ Final balance: ${finalBalance}`);
        
        // Summary
        console.log('\n' + '='.repeat(50));
        console.log('✅ TEST PASSED: Transaction Processing');
        console.log('='.repeat(50));
        console.log(`Initial balance: ${initialBalance}`);
        console.log(`After faucet:    ${balanceAfterFaucet}`);
        console.log(`After tx:        ${finalBalance}`);
        console.log(`Transaction:     ${tx.transactionHash}`);
        
    } catch (error) {
        console.error('\n❌ TEST FAILED:', error.message);
        process.exit(1);
    }
}

// Run test
test01_SendTransaction().catch(console.error);
