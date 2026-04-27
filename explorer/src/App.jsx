import { useState, useEffect } from 'react';
import './App.css';

function App() {
  const [status, setStatus] = useState(null);
  const [mempool, setMempool] = useState(null);
  const [loading, setLoading] = useState(true);

  const fetchStatus = async () => {
    try {
      const statusRes = await fetch('http://localhost:9933/status');
      const statusData = await statusRes.json();
      setStatus(statusData);

      const mempoolRes = await fetch('http://localhost:9933/mempool');
      const mempoolData = await mempoolRes.json();
      setMempool(mempoolData);
      
      setLoading(false);
    } catch (e) {
      console.error(e);
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchStatus();
    const interval = setInterval(fetchStatus, 3000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div style={{ padding: '2rem', fontFamily: 'system-ui, sans-serif' }}>
      <header style={{ marginBottom: '2rem', borderBottom: '1px solid #ccc', paddingBottom: '1rem' }}>
        <h1 style={{ margin: 0 }}>🌌 Nebula Block Explorer</h1>
        <p style={{ color: '#666' }}>Real-time overview of the testnet</p>
      </header>

      {loading ? (
        <p>Loading chain data...</p>
      ) : (
        <div style={{ display: 'flex', gap: '2rem' }}>
          <section style={{ flex: 1, backgroundColor: '#f9f9f9', padding: '1.5rem', borderRadius: '8px' }}>
            <h2>Network Status</h2>
            {status ? (
              <ul style={{ listStyle: 'none', padding: 0 }}>
                <li><strong>Chain Height:</strong> {status.height}</li>
                <li><strong>Finalized Height:</strong> {status.finalized_height || 0}</li>
                <li><strong>Mempool Tx Count:</strong> {status.mempool_size}</li>
              </ul>
            ) : (
              <p style={{color: 'red'}}>Node offline</p>
            )}
          </section>

          <section style={{ flex: 1, backgroundColor: '#f9f9f9', padding: '1.5rem', borderRadius: '8px' }}>
            <h2>Mempool Transactions</h2>
            {mempool && mempool.transactions.length > 0 ? (
              <ul style={{ paddingLeft: '1rem' }}>
                {mempool.transactions.map((tx, idx) => (
                  <li key={idx}>Nonce: {tx.nonce} | Gas: {tx.gas_limit}</li>
                ))}
              </ul>
            ) : (
              <p>No pending transactions</p>
            )}
          </section>
        </div>
      )}
    </div>
  );
}

export default App;
