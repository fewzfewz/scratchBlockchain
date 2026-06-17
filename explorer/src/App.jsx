import { useState } from 'react';
import './App.css';
import DashboardPage from './DashboardPage';
import ValidatorsPage from './ValidatorsPage';
import StakingPage from './StakingPage';

const TABS = [
  { id: 'dashboard', label: 'Dashboard' },
  { id: 'validators', label: 'Validators' },
  { id: 'staking', label: 'Staking' },
];

function App() {
  const [activeTab, setActiveTab] = useState('dashboard');

  return (
    <div className="app-shell">
      <div className="ambient ambient-left" />
      <div className="ambient ambient-right" />

      <main className="dashboard">
        <section className="hero">
          <div className="hero-copy">
            <p className="eyebrow">Scratch Blockchain</p>
            <h1>Nebula Explorer</h1>
            <p className="hero-text">
              A live control room for network activity, validators, and staking.
            </p>
          </div>
        </section>

        <nav className="tab-bar">
          {TABS.map((tab) => (
            <button
              key={tab.id}
              className={`tab-button ${activeTab === tab.id ? 'active' : ''}`}
              onClick={() => setActiveTab(tab.id)}
            >
              {tab.label}
            </button>
          ))}
        </nav>

        <div className="content-grid">
          {activeTab === 'dashboard' && <DashboardPage />}
          {activeTab === 'validators' && <ValidatorsPage />}
          {activeTab === 'staking' && <StakingPage />}
        </div>
      </main>
    </div>
  );
}

export default App;
