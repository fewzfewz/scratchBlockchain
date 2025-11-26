const { ModularClient, HttpProvider } = require('@modular-blockchain/sdk');

async function test02_StateQueries() {
    console.log('🧪 Test 1.4: State Queries\n');
    
    try {
        const provider = new HttpProvider('http://localhost/rpc');
        const client = new ModularClient(provider);
        
        console.log('1. Testing chain queries...');
        const chainId = await client.getChainId();
        const blockNumber = await client.getBlockNumber();
        console.log(`   ✅ Chain ID: ${chainId}`);
        console.log(`   ✅ Block Number: ${blockNumber}`);
        
        console.log('\n2. Testing block queries...');
        const block = await client.getLatestBlock();
        console.log(`   ✅ Latest Block: ${block.number}`);
        console.log(`   ✅ Block Hash: ${block.hash}`);
        console.log(`   ✅ Transactions: ${block.transactions.length}`);
        
        console.log('\n3. Testing account queries...');
        const testAddress = '0x0000000000000000000000000000000000000001';
        const balance = await client.getBalance(testAddress);
        const nonce = await client.getNonce(testAddress);
        console.log(`   ✅ Balance: ${balance}`);
        console.log(`   ✅ Nonce: ${nonce}`);
        
        console.log('\n' + '='.repeat(50));
        console.log('✅ TEST PASSED: State Queries');
        console.log('='.repeat(50));
        
    } catch (error) {
        console.error('\n❌ TEST FAILED:', error.message);
        process.exit(1);
    }
}

test02_StateQueries().catch(console.error);
