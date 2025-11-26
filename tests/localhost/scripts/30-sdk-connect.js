const { ModularClient, HttpProvider, Wallet } = require('@modular-blockchain/sdk');

async function test30_SDKConnect() {
    console.log('🧪 Test 9.1: SDK Connection\n');
    
    try {
        console.log('1. Creating HTTP provider...');
        const provider = new HttpProvider('http://localhost/rpc', {
            timeout: 30000
        });
        console.log('   ✅ Provider created');
        
        console.log('\n2. Creating client...');
        const client = new ModularClient(provider);
        console.log('   ✅ Client created');
        
        console.log('\n3. Connecting to testnet...');
        await client.connect();
        console.log('   ✅ Connected successfully');
        
        console.log('\n4. Getting chain information...');
        const chainId = await client.getChainId();
        const blockNumber = await client.getBlockNumber();
        console.log(`   ✅ Chain ID: ${chainId}`);
        console.log(`   ✅ Block Number: ${blockNumber}`);
        
        console.log('\n5. Testing connection status...');
        const isConnected = client.isConnected();
        console.log(`   ✅ Connection status: ${isConnected}`);
        
        console.log('\n6. Getting latest block...');
        const block = await client.getLatestBlock();
        console.log(`   ✅ Block ${block.number} retrieved`);
        console.log(`   ✅ Block hash: ${block.hash}`);
        
        console.log('\n' + '='.repeat(50));
        console.log('✅ TEST PASSED: SDK Connection');
        console.log('='.repeat(50));
        console.log('SDK is working correctly!');
        console.log(`Connected to: ${chainId}`);
        console.log(`Current block: ${blockNumber}`);
        
    } catch (error) {
        console.error('\n❌ TEST FAILED:', error.message);
        console.error('\nTroubleshooting:');
        console.error('1. Check testnet is running: docker-compose ps');
        console.error('2. Check RPC endpoint: curl http://localhost/rpc/status');
        console.error('3. Check logs: docker-compose logs rpc1');
        process.exit(1);
    }
}

test30_SDKConnect().catch(console.error);
