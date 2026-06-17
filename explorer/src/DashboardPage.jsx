import { useEffect, useState } from 'react';

const API_URL = 'http://localhost:9933';

const formatNumber = (value) => {
  if (value === null || value === undefined || Number.isNaN(Number(value))) return '--';
  return Number(value).toLocaleString();
};

const shortenValue = (value, start = 10, end = 8) => {
  if (!value) return '--';
  if (value.length <= start + end + 3) return value;
  return `${value.slice(0, start)}...${value.slice(-end)}`;
};

function DashboardPage() {
  const [status, setStatus] = useState(null);
  const [mempool, setMempool] = useState(null);
  const [loading, setLoading] = useState(true);
  const [lastUpdated, setLastUpdated] = useState(null);
  const [error, setError] = useState('');

  useEffect(() => {
    let active = true;

    const fetchDashboard = async () => {
      try {
        const [statusRes, mempoolRes] = await Promise.all([
          fetch(`${API_URL}/status`),
          fetch(`${API_URL}/mempool`),
        ]);

        if (!statusRes.ok || !mempoolRes.ok) {
          throw new Error('Node returned an unexpected response.');
        }

        const [statusData, mempoolData] = await Promise.all([
          statusRes.json(),
          mempoolRes.json(),
        ]);

        if (!active) return;

        setStatus(statusData);
        setMempool(mempoolData);
        setLastUpdated(new Date());
        setError('');
      } catch (fetchError) {
        if (!active) return;
        setError('Unable to reach the local node at http://localhost:9933.');
      } finally {
        if (active) setLoading(false);
      }
    };

    fetchDashboard();
    const interval = setInterval(fetchDashboard, 3000);
    return () => { active = false; clearInterval(interval); };
  }, []);

  const transactions = mempool?.transactions ?? [];
  const finalizedHeight = status?.finalized_height ?? 0;
  const currentHeight = status?.height ?? 0;
  const finalityGap = Math.max(currentHeight - finalizedHeight, 0);

  return (
    <>
      <div className="hero-status">
        <div className="status-pill">
          <span className={`status-dot ${error ? 'offline' : 'online'}`} />
          {error ? 'Node offline' : 'Node connected'}
        </div>
        <p>{lastUpdated ? `Updated ${lastUpdated.toLocaleTimeString()}` : 'Waiting for data'}</p>
      </div>

      <section className="stat-grid">
        <article className="stat-card">
          <span className="stat-label">Chain Height</span>
          <strong>{formatNumber(currentHeight)}</strong>
          <p>Current tip reported by the local RPC.</p>
        </article>
        <article className="stat-card">
          <span className="stat-label">Finalized Height</span>
          <strong>{formatNumber(finalizedHeight)}</strong>
          <p>Latest block considered finalized by the node.</p>
        </article>
        <article className="stat-card">
          <span className="stat-label">Mempool Load</span>
          <strong>{formatNumber(status?.mempool_size ?? transactions.length)}</strong>
          <p>Pending transactions waiting for inclusion.</p>
        </article>
        <article className="stat-card">
          <span className="stat-label">Finality Gap</span>
          <strong>{formatNumber(finalityGap)}</strong>
          <p>Distance between chain tip and finalized state.</p>
        </article>
      </section>

      <section className="content-grid">
        <article className="panel panel-wide">
          <div className="panel-header">
            <div>
              <p className="panel-kicker">Network State</p>
              <h2>Chain snapshot</h2>
            </div>
          </div>

          <div className="metrics-list">
            <div className="metric-row">
              <span>RPC endpoint</span>
              <code>{API_URL}</code>
            </div>
            <div className="metric-row">
              <span>Synchronization</span>
              <strong>{error ? 'Disconnected' : 'Healthy'}</strong>
            </div>
            <div className="metric-row">
              <span>Pending capacity</span>
              <strong>
                {transactions.length > 0
                  ? `${formatNumber(transactions.length)} tx visible`
                  : 'Queue is currently clear'}
              </strong>
            </div>
          </div>

          {error ? (
            <div className="empty-state error-state">
              <h3>Connection lost</h3>
              <p>{error}</p>
            </div>
          ) : (
            <div className="callout">
              <span className="callout-label">Operator note</span>
              <p>
                This view refreshes every three seconds, so it works well as a quick
                health check while you run local nodes, tests, or faucet requests.
              </p>
            </div>
          )}
        </article>

        <article className="panel">
          <div className="panel-header">
            <div>
              <p className="panel-kicker">Pending Flow</p>
              <h2>Mempool transactions</h2>
            </div>
          </div>

          {loading ? (
            <div className="empty-state">
              <h3>Loading data</h3>
              <p>Waiting for the local node to answer.</p>
            </div>
          ) : transactions.length > 0 ? (
            <div className="transaction-list">
              {transactions.slice(0, 8).map((tx, index) => (
                <div className="transaction-row" key={`${tx.nonce}-${index}`}>
                  <div>
                    <span className="transaction-title">Nonce {formatNumber(tx.nonce)}</span>
                    <p>{shortenValue(tx.to || tx.recipient || tx.hash || 'Pending payload')}</p>
                  </div>
                  <div className="transaction-meta">
                    <strong>{formatNumber(tx.gas_limit ?? tx.gas ?? 0)}</strong>
                    <span>gas limit</span>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="empty-state">
              <h3>No pending transactions</h3>
              <p>The queue is empty and ready for the next workload.</p>
            </div>
          )}
        </article>
      </section>
    </>
  );
}

export default DashboardPage;
