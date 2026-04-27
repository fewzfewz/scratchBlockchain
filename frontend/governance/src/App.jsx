const proposals = [
  {
    id: 'GOV-014',
    title: 'Raise validator set target to 48',
    status: 'Active',
    turnout: '61%',
    closesIn: '18h',
  },
  {
    id: 'GOV-013',
    title: 'Reduce faucet cooldown for local testnet',
    status: 'Review',
    turnout: '32%',
    closesIn: '2d',
  },
  {
    id: 'GOV-012',
    title: 'Enable bridge circuit breaker alerts',
    status: 'Passed',
    turnout: '84%',
    closesIn: 'Finalized',
  },
];

const treasury = [
  { label: 'Treasury reserve', value: '12.4M NBL' },
  { label: 'Voting power online', value: '78.2%' },
  { label: 'Open proposals', value: '03' },
];

function App() {
  return (
    <div className="gov-shell">
      <div className="ambient ambient-left" />
      <div className="ambient ambient-right" />

      <main className="gov-app">
        <section className="hero">
          <div>
            <p className="eyebrow">Scratch Blockchain</p>
            <h1>Nebula Governance</h1>
            <p className="hero-copy">
              A clean control surface for proposal review, turnout tracking, and
              treasury context while the full governance flow matures.
            </p>
          </div>

          <div className="hero-card">
            <span className="hero-label">Network posture</span>
            <strong>Deliberation window open</strong>
            <p>Use this frontend as the landing shell for proposal and voting workflows.</p>
          </div>
        </section>

        <section className="stats-grid">
          {treasury.map((item) => (
            <article key={item.label} className="stat-card">
              <span>{item.label}</span>
              <strong>{item.value}</strong>
            </article>
          ))}
        </section>

        <section className="content-grid">
          <article className="panel">
            <div className="panel-header">
              <div>
                <p className="panel-kicker">Proposal Queue</p>
                <h2>Current proposals</h2>
              </div>
              <button type="button">New proposal</button>
            </div>

            <div className="proposal-list">
              {proposals.map((proposal) => (
                <div className="proposal-card" key={proposal.id}>
                  <div>
                    <span className="proposal-id">{proposal.id}</span>
                    <h3>{proposal.title}</h3>
                  </div>
                  <div className="proposal-meta">
                    <span>{proposal.status}</span>
                    <span>{proposal.turnout} turnout</span>
                    <span>{proposal.closesIn}</span>
                  </div>
                </div>
              ))}
            </div>
          </article>

          <article className="panel side-panel">
            <p className="panel-kicker">Voting Preview</p>
            <h2>Decision frame</h2>
            <div className="vote-stack">
              <div className="vote-bar">
                <span className="fill yes" style={{ width: '64%' }} />
              </div>
              <div className="vote-row">
                <strong>64% Yes</strong>
                <span>24% No</span>
                <span>12% Abstain</span>
              </div>
            </div>
            <p className="side-copy">
              The new governance app now has a working React entry point and a composed
              shell for treasury, proposals, and vote-state widgets.
            </p>
          </article>
        </section>
      </main>
    </div>
  );
}

export default App;
