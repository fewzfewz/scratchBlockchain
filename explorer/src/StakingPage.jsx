import { useEffect, useState } from 'react';

const API_URL = 'http://localhost:9933';

const formatNumber = (value) => {
  if (value === null || value === undefined || Number.isNaN(Number(value))) return '--';
  return Number(value).toLocaleString();
};

const formatStake = (stake) => {
  if (!stake) return '--';
  const n = Number(stake) / 1e18;
  return n.toFixed(2) + ' NBL';
};

function StakingPage() {
  const [stats, setStats] = useState(null);
  const [delegations, setDelegations] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [address, setAddress] = useState('');

  useEffect(() => {
    let active = true;

    const fetchData = async () => {
      try {
        const [statusRes, validatorsRes] = await Promise.all([
          fetch(`${API_URL}/status`),
          fetch(`${API_URL}/validators`),
        ]);

        if (!statusRes.ok || !validatorsRes.ok) throw new Error('Failed to fetch');

        const statusData = await statusRes.json();
        const validatorsData = await validatorsRes.json();
        const validators = Array.isArray(validatorsData) ? validatorsData : validatorsData.validators || [];

        if (!active) return;

        const totalStake = validators.reduce((sum, v) => sum + Number(v.stake || 0), 0);
        const activeCount = validators.filter((v) => v.is_active !== false).length;

        setStats({
          totalValidators: validators.length,
          activeValidators: activeCount,
          totalStake,
          avgCommission: validators.length > 0
            ? validators.reduce((s, v) => s + (v.commission_rate || 0.1), 0) / validators.length
            : 0,
        });
        setError('');
      } catch (err) {
        if (!active) return;
        setError('Unable to fetch staking data.');
      } finally {
        if (active) setLoading(false);
      }
    };

    fetchData();
    const interval = setInterval(fetchData, 15000);
    return () => { active = false; clearInterval(interval); };
  }, []);

  const fetchDelegations = async () => {
    if (!address) return;
    try {
      const res = await fetch(`${API_URL}/delegations/${address}`);
      if (!res.ok) throw new Error('No delegations found');
      const data = await res.json();
      setDelegations(Array.isArray(data) ? data : []);
    } catch {
      setDelegations([]);
    }
  };

  if (loading) {
    return (
      <section className="panel" style={{ gridColumn: '1 / -1' }}>
        <div className="empty-state">
          <h3>Loading staking data</h3>
          <p>Fetching network staking information...</p>
        </div>
      </section>
    );
  }

  return (
    <>
      <section className="stat-grid" style={{ gridColumn: '1 / -1' }}>
        <article className="stat-card">
          <span className="stat-label">Active Validators</span>
          <strong>{formatNumber(stats?.activeValidators)}</strong>
          <p>Of {formatNumber(stats?.totalValidators)} total registered</p>
        </article>
        <article className="stat-card">
          <span className="stat-label">Total Staked</span>
          <strong>{formatStake(stats?.totalStake)}</strong>
          <p>Network bonded stake</p>
        </article>
        <article className="stat-card">
          <span className="stat-label">Avg Commission</span>
          <strong>{stats ? (stats.avgCommission * 100).toFixed(1) + '%' : '--'}</strong>
          <p>Across all validators</p>
        </article>
        <article className="stat-card">
          <span className="stat-label">Inflation Rate</span>
          <strong>~5.2%</strong>
          <p>Current annual rate</p>
        </article>
      </section>

      <section className="panel">
        <div className="panel-header">
          <div>
            <p className="panel-kicker">Delegation</p>
            <h2>Check Delegations</h2>
          </div>
        </div>
        <div className="delegation-search">
          <input
            type="text"
            placeholder="Enter address (0x...)"
            value={address}
            onChange={(e) => setAddress(e.target.value)}
            className="delegation-input"
          />
          <button onClick={fetchDelegations} className="delegation-button">
            Check
          </button>
        </div>
        {delegations.length > 0 ? (
          <div className="metrics-list">
            {delegations.map((d, i) => (
              <div className="metric-row" key={i}>
                <span>Validator: {d.validator_address || d.validator || '--'}</span>
                <strong>{formatStake(d.amount)}</strong>
              </div>
            ))}
          </div>
        ) : address ? (
          <div className="empty-state">
            <h3>No delegations found</h3>
            <p>This address has no active delegations.</p>
          </div>
        ) : (
          <div className="empty-state">
            <h3>Enter an address</h3>
            <p>Enter a wallet address to check its delegations and pending rewards.</p>
          </div>
        )}
      </section>

      <section className="panel">
        <div className="panel-header">
          <div>
            <p className="panel-kicker">Rewards</p>
            <h2>Estimate Rewards</h2>
          </div>
        </div>
        <div className="empty-state">
          <h3>Rewards estimation coming soon</h3>
          <p>
            Future versions will show estimated APR, pending rewards, and historical
            reward payouts per validator and delegator.
          </p>
        </div>
      </section>
    </>
  );
}

export default StakingPage;
