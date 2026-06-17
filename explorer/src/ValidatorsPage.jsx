import { useEffect, useState } from 'react';

const API_URL = 'http://localhost:9933';

const shortenValue = (value, start = 10, end = 8) => {
  if (!value) return '--';
  if (value.length <= start + end + 3) return value;
  return `${value.slice(0, start)}...${value.slice(-end)}`;
};

const formatNumber = (value) => {
  if (value === null || value === undefined || Number.isNaN(Number(value))) return '--';
  return Number(value).toLocaleString();
};

const formatStake = (stake) => {
  if (!stake) return '--';
  const n = Number(stake) / 1e18;
  return n.toFixed(2) + ' NBL';
};

function ValidatorsPage() {
  const [validators, setValidators] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [selectedValidator, setSelectedValidator] = useState(null);

  useEffect(() => {
    let active = true;

    const fetchValidators = async () => {
      try {
        const res = await fetch(`${API_URL}/validators`);
        if (!res.ok) throw new Error('Failed to fetch validators');
        const data = await res.json();
        if (!active) return;
        setValidators(Array.isArray(data) ? data : data.validators || []);
        setError('');
      } catch (err) {
        if (!active) return;
        setError('Unable to fetch validator data.');
      } finally {
        if (active) setLoading(false);
      }
    };

    fetchValidators();
    const interval = setInterval(fetchValidators, 5000);
    return () => { active = false; clearInterval(interval); };
  }, []);

  if (loading) {
    return (
      <section className="panel" style={{ gridColumn: '1 / -1' }}>
        <div className="empty-state">
          <h3>Loading validators</h3>
          <p>Fetching validator set from the node...</p>
        </div>
      </section>
    );
  }

  if (error) {
    return (
      <section className="panel" style={{ gridColumn: '1 / -1' }}>
        <div className="empty-state error-state">
          <h3>Connection lost</h3>
          <p>{error}</p>
        </div>
      </section>
    );
  }

  if (selectedValidator) {
    const v = selectedValidator;
    return (
      <section className="panel panel-wide">
        <div className="panel-header">
          <div>
            <p className="panel-kicker">Validator Detail</p>
            <h2>{shortenValue(v.address || v.public_key, 8, 8)}</h2>
          </div>
          <button className="back-button" onClick={() => setSelectedValidator(null)}>
            &larr; Back
          </button>
        </div>
        <div className="metrics-list">
          <div className="metric-row">
            <span>Address</span>
            <code>{v.address || '--'}</code>
          </div>
          <div className="metric-row">
            <span>Public Key</span>
            <code>{shortenValue(v.public_key, 16, 16)}</code>
          </div>
          <div className="metric-row">
            <span>Stake</span>
            <strong>{formatStake(v.stake)}</strong>
          </div>
          <div className="metric-row">
            <span>Status</span>
            <strong style={{ color: v.is_active !== false ? '#4ade80' : '#f87171' }}>
              {v.is_active !== false ? 'Active' : 'Inactive'}
            </strong>
          </div>
          <div className="metric-row">
            <span>Commission Rate</span>
            <strong>{(v.commission_rate || 0.1) * 100}%</strong>
          </div>
          <div className="metric-row">
            <span>Blocks Produced</span>
            <strong>{formatNumber(v.blocks_produced)}</strong>
          </div>
          <div className="metric-row">
            <span>Blocks Missed</span>
            <strong style={{ color: (v.blocks_missed || 0) > 10 ? '#f87171' : '#94a3b8' }}>
              {formatNumber(v.blocks_missed)}
            </strong>
          </div>
          <div className="metric-row">
            <span>Delegators</span>
            <strong>{formatNumber(v.delegator_count)}</strong>
          </div>
          <div className="metric-row">
            <span>Total Delegated</span>
            <strong>{formatStake(v.total_delegated)}</strong>
          </div>
          <div className="metric-row">
            <span>Uptime</span>
            <strong>
              {v.blocks_produced && v.blocks_missed
                ? ((v.blocks_produced / (v.blocks_produced + v.blocks_missed)) * 100).toFixed(2) + '%'
                : '--'}
            </strong>
          </div>
        </div>
      </section>
    );
  }

  const columnHeaders = ['Rank', 'Validator', 'Stake', 'Commission', 'Status', 'Uptime', 'Delegators'];

  return (
    <section className="panel panel-wide">
      <div className="panel-header">
        <div>
          <p className="panel-kicker">Network</p>
          <h2>Active Validators ({validators.length})</h2>
        </div>
      </div>

      {validators.length === 0 ? (
        <div className="empty-state">
          <h3>No validators found</h3>
          <p>The validator set is empty or the node does not expose this endpoint.</p>
        </div>
      ) : (
        <div className="validator-table-wrapper">
          <table className="validator-table">
            <thead>
              <tr>
                {columnHeaders.map((h) => (
                  <th key={h}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {validators.map((v, i) => (
                <tr
                  key={v.address || v.public_key || i}
                  onClick={() => setSelectedValidator(v)}
                  className="validator-row"
                >
                  <td className="rank-cell">{i + 1}</td>
                  <td>
                    <code>{shortenValue(v.address || v.public_key, 8, 8)}</code>
                  </td>
                  <td className="number-cell">{formatStake(v.stake)}</td>
                  <td>{(v.commission_rate || 0.1) * 100}%</td>
                  <td>
                    <span className={`status-badge ${v.is_active !== false ? 'active' : 'inactive'}`}>
                      {v.is_active !== false ? 'Active' : 'Inactive'}
                    </span>
                  </td>
                  <td className="number-cell">
                    {v.blocks_produced && v.blocks_missed
                      ? ((v.blocks_produced / (v.blocks_produced + v.blocks_missed)) * 100).toFixed(1) + '%'
                      : '--'}
                  </td>
                  <td className="number-cell">{formatNumber(v.delegator_count)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}

export default ValidatorsPage;
