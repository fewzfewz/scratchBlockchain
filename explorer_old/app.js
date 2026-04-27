const API_URL = 'http://localhost:9933';

// State
let currentBlockHeight = 0;
let tps = 0;
let peerCount = 0;

// DOM Elements
const els = {
    blockHeight: document.getElementById('blockHeight'),
    tps: document.getElementById('tps'),
    peerCount: document.getElementById('peerCount'),
    lastBlockTime: document.getElementById('lastBlockTime'),
    blocksTable: document.getElementById('blocksTable'),
    txTable: document.getElementById('txTable'),
    searchInput: document.getElementById('searchInput')
};

// Formatters
const formatHash = (hash) => `${hash.substring(0, 6)}...${hash.substring(hash.length - 4)}`;
const formatTime = (timestamp) => moment(timestamp).fromNow();

// Fetch Data
async function fetchStatus() {
    try {
        const response = await fetch(`${API_URL}/status`);
        const data = await response.json();
        
        // Update Stats
        updateStat(els.blockHeight, data.block_height);
        updateStat(els.peerCount, data.peer_count);
        updateStat(els.tps, Math.floor(Math.random() * 50) + 100); // Mock TPS for now as API might not return it directly yet
        els.lastBlockTime.textContent = 'Just now'; // Real implementation would use block timestamp

        // If new block, fetch details
        if (data.block_height > currentBlockHeight) {
            currentBlockHeight = data.block_height;
            fetchLatestBlocks();
        }
    } catch (error) {
        console.error('Error fetching status:', error);
    }
}

async function fetchLatestBlocks() {
    try {
        // Fetch last 5 blocks
        const blocks = [];
        for (let i = 0; i < 5; i++) {
            const height = currentBlockHeight - i;
            if (height < 0) break;
            
            const response = await fetch(`${API_URL}/block/${height}`);
            if (response.ok) {
                const block = await response.json();
                blocks.push(block);
            }
        }
        
        renderBlocks(blocks);
        renderTransactions(blocks); // Extract txs from blocks
    } catch (error) {
        console.error('Error fetching blocks:', error);
    }
}

// Render Functions
function renderBlocks(blocks) {
    els.blocksTable.innerHTML = blocks.map(block => `
        <tr>
            <td><a href="#" class="hash">${block.header.slot}</a></td>
            <td><a href="#" class="hash">${formatHash(block.header.parent_hash)}</a></td> <!-- Using parent hash as ID for now -->
            <td>${block.transactions.length}</td>
            <td>Just now</td>
        </tr>
    `).join('');
}

function renderTransactions(blocks) {
    // Flatten transactions from all blocks
    const txs = blocks.flatMap(b => b.transactions).slice(0, 5);
    
    if (txs.length === 0) {
        els.txTable.innerHTML = '<tr><td colspan="4">No recent transactions</td></tr>';
        return;
    }

    els.txTable.innerHTML = txs.map(tx => `
        <tr>
            <td><a href="#" class="hash">${formatHash('0x' + Array.from(tx.signature).map(b => b.toString(16).padStart(2, '0')).join(''))}</a></td>
            <td><a href="#" class="hash">0x...</a></td>
            <td><a href="#" class="hash">0x...</a></td>
            <td>${tx.value}</td>
        </tr>
    `).join('');
}

function updateStat(element, newValue) {
    if (element.textContent != newValue) {
        element.textContent = newValue;
        element.classList.add('updated');
        setTimeout(() => element.classList.remove('updated'), 500);
    }
}

// Search Handler
els.searchInput.addEventListener('keypress', async (e) => {
    if (e.key === 'Enter') {
        const query = e.target.value.trim();
        // Implement search logic here (redirect to block/tx page)
        alert(`Search feature coming soon for: ${query}`);
    }
});

// Init
function init() {
    fetchStatus();
    setInterval(fetchStatus, 3000); // Poll every 3s
}

init();
