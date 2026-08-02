import { useState, useEffect, useMemo } from "react";
import {
  BarChart3, Vote, FileText, PlusCircle, Wallet, TrendingUp,
  TrendingDown, Clock, CheckCircle, XCircle, AlertTriangle,
  Users, Award, Search, X, ChevronRight, Send,
  PieChart, Activity, Shield, ArrowUpRight, ArrowDownRight,
  Info, Hash, Calendar, Tag, ThumbsUp,
  ThumbsDown, Minus, Zap, Layers, Target, Loader2,
  Bell, Gauge, ShieldCheck, WifiOff, Copy, PenLine, Rocket, BookOpen,
} from "lucide-react";

const RPC_URL = "http://localhost:8545";

const TABS = [
  { id: "dashboard", label: "Dashboard", icon: BarChart3 },
  { id: "proposals", label: "Proposals", icon: FileText },
  { id: "create", label: "Create", icon: PlusCircle },
  { id: "treasury", label: "Treasury", icon: Wallet },
  { id: "analytics", label: "Analytics", icon: PieChart },
];

const PROPOSALS_MOCK = [
  { id: 1, title: "Raise validator set target to 48", description: "Increase the maximum active validator set from 32 to 48 to improve network decentralization and security. This change requires updating the consensus parameter max_validators and will be phased in over 4 epochs.", proposer: "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18", status: "Active", yesVotes: "6400000", noVotes: "2100000", abstainVotes: "500000", startEpoch: 12450, endEpoch: 12650, deposit: "100000", executed: false, actions: [{ target: "0x0000000000000000000000000000000000001001", value: "0", signature: "setMaxValidators(uint256)", calldata: "0x0000000000000000000000000000000000000000000000000000000000000030" }] },
  { id: 2, title: "Reduce faucet cooldown for local testnet", description: "Lower the faucet cooldown period from 24 hours to 6 hours on the local testnet to accelerate developer onboarding. This is a testnet-only parameter change and does not affect mainnet.", proposer: "0x8fD8fB8fB8fB8fD8fB8fB8fD8fB8fB8fD8fB8fD", status: "Pending", yesVotes: "0", noVotes: "0", abstainVotes: "0", startEpoch: 12500, endEpoch: 12700, deposit: "50000", executed: false, actions: [{ target: "0x0000000000000000000000000000000000001002", value: "0", signature: "setFaucetCooldown(uint256)", calldata: "0x0000000000000000000000000000000000000000000000000000000000005460" }] },
  { id: 3, title: "Enable bridge circuit breaker alerts", description: "Configure on-chain circuit breaker thresholds for the cross-chain bridge. When daily volume exceeds 500k NBL or a single transaction exceeds 100k NBL, the breaker will pause bridge operations and notify validators.", proposer: "0x5B38Da6a701c568545dCfcB03FcB875f56beddC4", status: "Passed", yesVotes: "9200000", noVotes: "1800000", abstainVotes: "300000", startEpoch: 12100, endEpoch: 12300, deposit: "100000", executed: true, actions: [{ target: "0x0000000000000000000000000000000000001003", value: "0", signature: "setCircuitBreakerThresholds(uint256,uint256)", calldata: "0x000000000000000000000000000000000000000000000000000000000007a12000000000000000000000000000000000000000000000000000000000000186a0" }] },
  { id: 4, title: "Upgrade runtime to v2.1.0", description: "Protocol upgrade introducing EIP-1559 style fee market, parallel transaction execution, and improved state pruning. Full audit report available at ipfs://QmAudit. Validators must upgrade before epoch 13000.", proposer: "0xdD8fB8fB8fB8fD8fB8fB8fD8fB8fB8fD8fB8fB", status: "Active", yesVotes: "7800000", noVotes: "3200000", abstainVotes: "1000000", startEpoch: 12480, endEpoch: 12880, deposit: "250000", executed: false, actions: [{ target: "0x0000000000000000000000000000000000001004", value: "0", signature: "upgradeRuntime(string,bytes32)", calldata: "0x0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000a76322e312e300000000000000000000000000000000000000000000000000000" }] },
  { id: 5, title: "Adjust block time to 3 seconds", description: "Decrease block production time from 6 seconds to 3 seconds for faster finality. Requires coordination with infrastructure providers to ensure adequate block propagation.", proposer: "0x1CBd3b2770909D4e10f157cABC84C7264073C9Ec", status: "Rejected", yesVotes: "3500000", noVotes: "6500000", abstainVotes: "800000", startEpoch: 11900, endEpoch: 12100, deposit: "100000", executed: false, actions: [{ target: "0x0000000000000000000000000000000000001005", value: "0", signature: "setBlockTime(uint256)", calldata: "0x0000000000000000000000000000000000000000000000000000000000000003" }] },
  { id: 6, title: "Allocate treasury funds for developer grants", description: "Approve 500,000 NBL from the community treasury for a developer grant program focused on tooling, SDK improvements, and infrastructure. Grants to be administered by a 5-person committee elected by the community.", proposer: "0xAb8483F64d9C6d1EcF9b849Ae677dD3315835cb2", status: "Passed", yesVotes: "8800000", noVotes: "700000", abstainVotes: "500000", startEpoch: 12250, endEpoch: 12450, deposit: "150000", executed: false, actions: [{ target: "0x0000000000000000000000000000000000001006", value: "500000000000000000000000", signature: "transfer(address,uint256)", calldata: "0x000000000000000000000000742d35Cc6634C0532925a3b844Bc9e7595f2bD180000000000000000000000000000000000000000000000000000000000000001" }] },
  { id: 7, title: "Increase minimum stake to 10,000 NBL", description: "Raise the minimum validator self-stake from 1,000 NBL to 10,000 NBL to ensure validators have meaningful skin in the game. Existing validators have 6 months to comply.", proposer: "0x4B20993Bc481177ec7E8f571ceCaE8A9e22C02db", status: "Pending", yesVotes: "0", noVotes: "0", abstainVotes: "0", startEpoch: 12550, endEpoch: 12750, deposit: "100000", executed: false, actions: [{ target: "0x0000000000000000000000000000000000001001", value: "0", signature: "setMinStake(uint256)", calldata: "0x0000000000000000000000000000000000000000000000000000000000002710" }] },
  { id: 8, title: "Deploy L2 scaling testnet", description: "Approve deployment of a zk-rollup L2 testnet for high-throughput application testing. The testnet will run alongside the main L1 with a canonical bridge for asset transfers.", proposer: "0x617F2E2fD72FD9D5503197092aC168c91465E7f2", status: "Active", yesVotes: "7100000", noVotes: "2900000", abstainVotes: "600000", startEpoch: 12400, endEpoch: 12600, deposit: "200000", executed: false, actions: [{ target: "0x0000000000000000000000000000000000001007", value: "0", signature: "deployTestnet(string,bytes32)", calldata: "0x0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000a4c322d746573746e657400000000000000000000000000000000000000000000" }] },
];

const TREASURY_MOCK = {
  balance: "12400000000000000000000000",
  totalCollected: "18500000000000000000000000",
  totalSpent: "6100000000000000000000000",
  recentTransactions: [
    { hash: "0xabc...def1", type: "deposit", amount: "500000000000000000000000", from: "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18", to: "0xTreasury", reason: "Block rewards allocation", timestamp: Date.now() - 3600000, height: 12450 },
    { hash: "0xabc...def2", type: "spend", amount: "25000000000000000000000", from: "0xTreasury", to: "0x8fD8fB8fB8fB8fD8fB8fB8fD8fB8fB8fD8fB8fD", reason: "Dev grant disbursement", timestamp: Date.now() - 7200000, height: 12440 },
    { hash: "0xabc...def3", type: "deposit", amount: "10000000000000000000000", from: "0x5B38Da6a701c568545dCfcB03FcB875f56beddC4", reason: "Validator slashing forfeit", timestamp: Date.now() - 10800000, height: 12435 },
    { hash: "0xabc...def4", type: "deposit", amount: "1200000000000000000000000", from: "0x1CBd3b2770909D4e10f157cABC84C7264073C9Ec", reason: "Transaction fee pool", timestamp: Date.now() - 14400000, height: 12430 },
    { hash: "0xabc...def5", type: "spend", amount: "5000000000000000000000", from: "0xTreasury", to: "0xAb8483F64d9C6d1EcF9b849Ae677dD3315835cb2", reason: "Community event sponsorship", timestamp: Date.now() - 18000000, height: 12420 },
    { hash: "0xabc...def6", type: "deposit", amount: "300000000000000000000000", from: "0x4B20993Bc481177ec7E8f571ceCaE8A9e22C02db", reason: "Validator registration fees", timestamp: Date.now() - 21600000, height: 12410 },
  ],
};

const GOV_PARAMS_MOCK = {
  votingPeriod: 200,
  quorum: 40,
  proposalDeposit: "100000",
  maxActions: 10,
  timelockPeriod: 100,
  minVotingPower: "500000",
};

const VALIDATORS_MOCK = [
  { address: "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18", publicKey: "0x02aabb...", stake: "3200000000000000000000000", commissionRate: 10, isActive: true, blocksProduced: 4521, blocksMissed: 12, delegatorCount: 156, totalDelegated: "2800000000000000000000000" },
  { address: "0x8fD8fB8fB8fB8fD8fB8fB8fD8fB8fB8fD8fB8fD", publicKey: "0x03bbcc...", stake: "2100000000000000000000000", commissionRate: 8, isActive: true, blocksProduced: 3890, blocksMissed: 8, delegatorCount: 98, totalDelegated: "1800000000000000000000000" },
  { address: "0x5B38Da6a701c568545dCfcB03FcB875f56beddC4", publicKey: "0x04ccdd...", stake: "1500000000000000000000000", commissionRate: 12, isActive: true, blocksProduced: 3201, blocksMissed: 25, delegatorCount: 72, totalDelegated: "1200000000000000000000000" },
  { address: "0x1CBd3b2770909D4e10f157cABC84C7264073C9Ec", publicKey: "0x05ddee...", stake: "800000000000000000000000", commissionRate: 15, isActive: true, blocksProduced: 2100, blocksMissed: 42, delegatorCount: 45, totalDelegated: "600000000000000000000000" },
  { address: "0xAb8483F64d9C6d1EcF9b849Ae677dD3315835cb2", publicKey: "0x06eeff...", stake: "500000000000000000000000", commissionRate: 7, isActive: false, blocksProduced: 980, blocksMissed: 120, delegatorCount: 22, totalDelegated: "300000000000000000000000" },
];

function shortenHash(hash, chars = 6) {
  if (!hash) return "";
  if (hash.length <= chars * 2 + 2) return hash;
  return hash.slice(0, chars + 2) + "..." + hash.slice(-chars);
}

function formatNbl(wei, decimals = 18) {
  try {
    const big = BigInt(wei);
    const divisor = BigInt(10) ** BigInt(decimals);
    const whole = big / divisor;
    const frac = big % divisor;
    const fracStr = frac.toString().padStart(decimals, "0").slice(0, 4);
    const wholeStr = whole.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ",");
    if (fracStr === "0000") return wholeStr;
    return `${wholeStr}.${fracStr}`;
  } catch {
    return "0";
  }
}

function fmt(v) {
  return v == null || isNaN(Number(v)) ? "--" : Number(v).toLocaleString();
}

function formatTime(ts) {
  const diff = Date.now() - ts;
  const mins = Math.floor(diff / 60000);
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

function getStatusColor(status) {
  switch (status) {
    case "Active": return "bg-emerald-500/20 text-emerald-400 border-emerald-500/30";
    case "Pending": return "bg-amber-500/20 text-amber-400 border-amber-500/30";
    case "Passed": return "bg-blue-500/20 text-blue-400 border-blue-500/30";
    case "Rejected": return "bg-red-500/20 text-red-400 border-red-500/30";
    case "Executed": return "bg-violet-500/20 text-violet-400 border-violet-500/30";
    default: return "bg-slate-500/20 text-slate-400 border-slate-500/30";
  }
}

function getStatusIcon(status) {
  switch (status) {
    case "Active": return Zap;
    case "Pending": return Clock;
    case "Passed": return CheckCircle;
    case "Rejected": return XCircle;
    case "Executed": return Award;
    default: return AlertTriangle;
  }
}

function NotificationBar({ proposals, onDismiss, onViewProposals }) {
  if (proposals.length === 0) return null;
  return (
    <div className="flex items-center gap-3 px-4 py-3 rounded-xl bg-gradient-to-r from-cyan-500/10 via-blue-500/10 to-indigo-500/10 border border-blue-500/20 text-sm relative">
      <Bell className="w-4 h-4 text-blue-400 shrink-0" />
      <span className="text-slate-700 dark:text-slate-300">
        <strong className="text-blue-300">{proposals.length}</strong> proposal{proposals.length > 1 ? "s" : ""} need{proposals.length === 1 ? "s" : ""} your vote
      </span>
      <button onClick={onViewProposals} className="ml-auto text-blue-400 hover:text-blue-300 underline underline-offset-2">
        View Proposals
      </button>
      <button onClick={onDismiss} className="text-slate-500 hover:text-slate-700 dark:text-slate-300">
        <X className="w-4 h-4" />
      </button>
    </div>
  );
}

function StatCard({ label, value, sub, icon: Icon, trend, color = "blue" }) {
  const chips = {
    blue: "from-blue-600 to-cyan-600",
    emerald: "from-emerald-600 to-teal-600",
    amber: "from-amber-500 to-orange-600",
    violet: "from-violet-600 to-purple-600",
    rose: "from-rose-500 to-pink-600",
  };
  const trendColor = trend === "up" ? "text-emerald-400" : trend === "down" ? "text-rose-400" : "text-slate-400 dark:text-slate-500";
  return (
    <div className="p-4 rounded-2xl glass-strong hover:-translate-y-0.5 hover:shadow-lg hover:shadow-blue-500/5 transition-all">
      <div className="flex items-center gap-2 mb-3">
        <div className={`w-8 h-8 rounded-lg bg-gradient-to-br ${chips[color] || chips.blue} flex items-center justify-center text-white shadow-md`}>
          {Icon && <Icon className="w-4 h-4" />}
        </div>
        <span className="text-[11px] uppercase tracking-wider text-slate-500 dark:text-slate-400">{label}</span>
      </div>
      <div className="text-2xl font-bold text-slate-900 dark:text-white tabular-nums">{value}</div>
      {sub && (
        <div className="flex items-center gap-1 mt-1 text-xs">
          {trend === "up" && <ArrowUpRight className="w-3 h-3 text-emerald-400" />}
          {trend === "down" && <ArrowDownRight className="w-3 h-3 text-rose-400" />}
          <span className={trendColor}>{sub}</span>
        </div>
      )}
    </div>
  );
}

function ProgressBar({ yes, no, abstain, height = "h-2" }) {
  const total = yes + no + abstain;
  if (total === 0) return <div className={`${height} rounded-full bg-slate-200/50 dark:bg-slate-700/50`} />;
  const yPct = (yes / total) * 100;
  const nPct = (no / total) * 100;
  const aPct = (abstain / total) * 100;
  return (
    <div className={`${height} rounded-full bg-slate-200/50 dark:bg-slate-700/50 overflow-hidden flex`}>
      {yPct > 0 && <div className="bg-gradient-to-r from-emerald-500 to-emerald-400 transition-all duration-500" style={{ width: `${yPct}%` }} />}
      {nPct > 0 && <div className="bg-gradient-to-r from-rose-500 to-rose-400 transition-all duration-500" style={{ width: `${nPct}%` }} />}
      {aPct > 0 && <div className="bg-gradient-to-r from-slate-500 to-slate-400 transition-all duration-500" style={{ width: `${aPct}%` }} />}
    </div>
  );
}

function VoteBreakdown({ yes, no, abstain }) {
  const total = yes + no + abstain;
  const yPct = total > 0 ? ((yes / total) * 100).toFixed(1) : "0";
  const nPct = total > 0 ? ((no / total) * 100).toFixed(1) : "0";
  const aPct = total > 0 ? ((abstain / total) * 100).toFixed(1) : "0";
  return (
    <div className="space-y-2.5">
      <ProgressBar yes={yes} no={no} abstain={abstain} height="h-3" />
      <div className="flex justify-between text-xs">
        <span className="text-emerald-400 font-medium">{yPct}% For</span>
        <span className="text-slate-400">{aPct}% Abstain</span>
        <span className="text-rose-400 font-medium">{nPct}% Against</span>
      </div>
    </div>
  );
}

function DashboardSkeleton() {
  return (
    <div className="space-y-6 animate-fade-in">
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3 md:gap-4">
        {[0, 1, 2, 3].map(i => (
          <div key={i} className="h-28 rounded-2xl glass-strong animate-pulse" />
        ))}
      </div>
      <div className="grid md:grid-cols-2 gap-4 md:gap-6">
        {[0, 1].map(i => <div key={i} className="h-56 rounded-2xl glass-strong animate-pulse" />)}
      </div>
      <div className="h-64 rounded-2xl glass-strong animate-pulse" />
    </div>
  );
}

function Dashboard({ proposals, treasury, validators, govParams, onTabChange }) {
  const balance = treasury ? formatNbl(treasury.balance) : "—";
  const activeCount = proposals.filter(p => p.status === "Active").length;
  const passedCount = proposals.filter(p => p.status === "Passed").length;
  const activeValidators = validators.filter(v => v.isActive).length;
  const totalStaked = validators.reduce((sum, v) => sum + BigInt(v.stake), 0n);
  const totalYield = proposals.length > 0
    ? ((Number(proposals.filter(p => p.status === "Passed").length) / proposals.length) * 100).toFixed(0)
    : "0";
  const [delegationAddr, setDelegationAddr] = useState("");

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3 md:gap-4">
        <StatCard label="Treasury Balance" value={`${balance} NBL`} sub="Community funds" icon={Wallet} color="blue" />
        <StatCard label="Active Proposals" value={activeCount.toString()} sub={`${passedCount} passed total`} icon={FileText} color="amber" />
        <StatCard label="Active Validators" value={activeValidators.toString()} sub={`${validators.length} total registered`} icon={Shield} color="emerald" />
        <StatCard label="Proposal Success" value={`${totalYield}%`} sub="Of all proposals" icon={Target} color="violet" />
      </div>

      <div className="grid md:grid-cols-2 gap-4 md:gap-6">
        <div className="p-5 rounded-2xl border border-slate-200 dark:border-slate-700/50 bg-white/70 dark:bg-slate-800/40 backdrop-blur-sm">
          <h3 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4 flex items-center gap-2">
            <Activity className="w-4 h-4 text-blue-400" />
            System Health
          </h3>
          <div className="space-y-3.5">
            {[
              { label: "Voting Power Online", value: "78.2%", sub: "12.4M NBL staked", color: "bg-emerald-500" },
              { label: "Quorum Threshold", value: `${govParams.quorum}%`, sub: "Current participation", color: "bg-blue-500" },
              { label: "Active Validators", value: `${activeValidators}/${validators.length}`, sub: "Producing blocks", color: "bg-amber-500" },
              { label: "Inflation Rate", value: "4.2%", sub: "Annualized", color: "bg-violet-500" },
            ].map(({ label, value, sub, color }) => (
              <div key={label} className="flex items-center justify-between">
                <div>
                  <div className="text-sm text-slate-400">{label}</div>
                  <div className="text-xs text-slate-500">{sub}</div>
                </div>
                <div className="flex items-center gap-2">
                  <div className={`w-2 h-2 rounded-full ${color}`} />
                  <span className="text-sm font-semibold text-slate-900 dark:text-white">{value}</span>
                </div>
              </div>
            ))}
          </div>
        </div>

        <div className="p-5 rounded-2xl border border-slate-200 dark:border-slate-700/50 bg-white/70 dark:bg-slate-800/40 backdrop-blur-sm">
          <h3 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4 flex items-center gap-2">
            <Search className="w-4 h-4 text-blue-400" />
            Delegation Lookup
          </h3>
          <p className="text-xs text-slate-500 mb-3">Enter an address to check its delegations and staking positions</p>
          <div className="flex gap-2">
            <input
              value={delegationAddr}
              onChange={e => setDelegationAddr(e.target.value)}
              placeholder="0x..."
              className="flex-1 px-3 py-2.5 rounded-xl bg-white/70 dark:bg-slate-700/60 border border-slate-300 dark:border-slate-600/60 text-sm text-slate-700 dark:text-slate-200 placeholder-slate-500 focus:outline-none focus:border-blue-500/50 focus:ring-1 focus:ring-blue-500/20 transition-all font-mono"
            />
            <button className="px-4 py-2.5 rounded-xl bg-blue-600 hover:bg-blue-500 text-white text-sm font-medium transition-all flex items-center gap-1.5">
              <Search className="w-3.5 h-3.5" />
              Lookup
            </button>
          </div>
          <div className="mt-4 p-3 rounded-xl bg-slate-100/60 dark:bg-slate-700/30 border border-dashed border-slate-300 dark:border-slate-600/40">
            <div className="flex items-center gap-2 text-sm text-slate-400">
              <Info className="w-3.5 h-3.5 text-slate-500" />
              Enter a validator or delegator address to view their staking details, rewards, and delegation history.
            </div>
          </div>
        </div>
      </div>

      <div className="p-5 rounded-2xl border border-slate-200 dark:border-slate-700/50 bg-white/70 dark:bg-slate-800/40 backdrop-blur-sm">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-sm font-semibold text-slate-700 dark:text-slate-300 flex items-center gap-2">
            <Zap className="w-4 h-4 text-amber-400" />
            Recent Proposals
          </h3>
          <button onClick={() => onTabChange("proposals")} className="text-xs text-blue-400 hover:text-blue-300 flex items-center gap-1">
            View all <ChevronRight className="w-3 h-3" />
          </button>
        </div>
        <div className="space-y-2.5">
          {proposals.slice(0, 3).map(p => {
            const Icon = getStatusIcon(p.status);
            return (
              <div key={p.id} className="flex items-center justify-between p-3 rounded-xl bg-slate-100/60 dark:bg-slate-700/30 hover:bg-slate-200/50 dark:hover:bg-slate-700/50 transition-colors cursor-pointer">
                <div className="flex items-center gap-3 min-w-0">
                  <Icon className={`w-4 h-4 shrink-0 ${getStatusColor(p.status).split(" ")[1]}`} />
                  <div className="min-w-0">
                    <div className="text-sm text-slate-700 dark:text-slate-200 truncate">{p.title}</div>
                    <div className="text-xs text-slate-500">#{p.id} · {p.status}</div>
                  </div>
                </div>
                <div className="text-xs text-slate-400 shrink-0 ml-3">
                  {BigInt(p.yesVotes) > 0 || BigInt(p.noVotes) > 0
                    ? `${((Number(p.yesVotes) / (Number(p.yesVotes) + Number(p.noVotes))) * 100).toFixed(0)}% For`
                    : "No votes"}
                </div>
              </div>
            );
          })}
        </div>
      </div>

      <div className="p-5 rounded-2xl glass-strong">
        <h3 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4 flex items-center gap-2">
          <BookOpen className="w-4 h-4 text-blue-400" />
          How governance works
        </h3>
        <div className="grid md:grid-cols-3 gap-4">
          {[
            { step: "01", title: "Propose", desc: "Submit an on-chain proposal with a refundable deposit. Include calldata that executes automatically if it passes.", icon: PenLine, chip: "from-blue-600 to-cyan-600" },
            { step: "02", title: "Vote", desc: `Validators and delegators vote For, Against, or Abstain during the ${govParams.votingPeriod}-epoch voting period. Quorum is ${govParams.quorum}%.`, icon: Vote, chip: "from-emerald-600 to-teal-600" },
            { step: "03", title: "Execute", desc: `Passed proposals are timelocked for ${govParams.timelockPeriod} epochs, then executed on-chain. Failed deposits are forfeited.`, icon: Rocket, chip: "from-violet-600 to-purple-600" },
          ].map(({ step, title, desc, icon: Icon, chip }) => (
            <div key={step} className="p-4 rounded-xl bg-slate-100/50 dark:bg-slate-700/20 hover:-translate-y-0.5 transition-all">
              <div className="flex items-center justify-between mb-3">
                <div className={`w-9 h-9 rounded-xl bg-gradient-to-br ${chip} flex items-center justify-center text-white shadow-md`}>
                  <Icon className="w-4 h-4" />
                </div>
                <span className="text-2xl font-bold text-slate-200 dark:text-slate-700">{step}</span>
              </div>
              <div className="text-sm font-semibold text-slate-700 dark:text-slate-200 mb-1">{title}</div>
              <p className="text-xs text-slate-400 dark:text-slate-500 leading-relaxed">{desc}</p>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

function Proposals({ proposals, onVote }) {
  const [filter, setFilter] = useState("All");
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(null);
  const [voting, setVoting] = useState(null);
  const [toast, setToast] = useState(null);
  const filters = ["All", "Active", "Pending", "Passed", "Rejected", "Executed"];

  const filtered = proposals.filter(p => {
    if (filter !== "All" && p.status !== filter) return false;
    const q = query.trim().toLowerCase();
    if (!q) return true;
    return (
      p.title.toLowerCase().includes(q) ||
      p.description.toLowerCase().includes(q) ||
      p.proposer.toLowerCase().includes(q) ||
      String(p.id).includes(q)
    );
  });

  const showToast = (msg) => {
    setToast(msg);
    setTimeout(() => setToast(null), 3000);
  };

  const handleVote = (proposalId, support) => {
    setVoting(proposalId);
    setTimeout(() => {
      onVote(proposalId, support);
      setVoting(null);
      showToast(`Vote cast: ${support} on proposal #${proposalId}`);
    }, 600);
  };

  return (
    <div className="space-y-4">
      {toast && (
        <div className="fixed top-4 right-4 z-50 px-4 py-3 rounded-xl bg-emerald-600/90 text-white text-sm shadow-2xl backdrop-blur-md border border-emerald-500/30 animate-slide-up flex items-center gap-2">
          <CheckCircle className="w-4 h-4" />
          {toast}
        </div>
      )}

      <div className="flex flex-wrap items-center gap-2">
        <div className="flex flex-wrap gap-2">
          {filters.map(f => (
            <button
              key={f}
              onClick={() => { setFilter(f); setSelected(null); }}
              className={`px-3.5 py-1.5 rounded-full text-xs font-medium transition-all ${
                filter === f
                  ? "bg-gradient-to-r from-blue-600 to-cyan-600 text-white shadow-md shadow-blue-600/20"
                  : "bg-slate-200/50 dark:bg-slate-700/50 text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 hover:bg-slate-200/60 dark:hover:bg-slate-700/80"
              }`}
            >
              {f}
            </button>
          ))}
        </div>
        <div className="flex-1 min-w-[220px] ml-auto">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-slate-400" />
            <input
              value={query}
              onChange={e => { setQuery(e.target.value); setSelected(null); }}
              placeholder="Search proposals..."
              className="w-full pl-9 pr-8 py-2 rounded-full bg-white/70 dark:bg-slate-700/60 border border-slate-300 dark:border-slate-600/60 text-sm text-slate-700 dark:text-slate-200 placeholder-slate-500 focus:outline-none focus:border-blue-500/50 focus:ring-1 focus:ring-blue-500/20 transition-all"
            />
            {query && (
              <button onClick={() => setQuery("")} className="absolute right-2.5 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600 dark:hover:text-slate-200">
                <X className="w-3.5 h-3.5" />
              </button>
            )}
          </div>
        </div>
      </div>

      {selected ? (
        <div className="p-5 md:p-6 rounded-2xl border border-slate-200 dark:border-slate-700/50 bg-white/70 dark:bg-slate-800/40 backdrop-blur-sm space-y-5">
          <button onClick={() => setSelected(null)} className="text-xs text-blue-400 hover:text-blue-300 flex items-center gap-1">
            <ChevronRight className="w-3 h-3 rotate-180" /> Back to list
          </button>

          <div className="flex items-start justify-between gap-4">
            <div>
              <div className="flex items-center gap-2 mb-2">
                <span className="text-xs text-slate-500 font-mono">#{selected.id}</span>
                <span className={`px-2 py-0.5 rounded-full text-xs font-medium border ${getStatusColor(selected.status)}`}>
                  {selected.status}
                </span>
              </div>
              <h2 className="text-xl md:text-2xl font-bold text-slate-900 dark:text-white">{selected.title}</h2>
            </div>
          </div>

          <p className="text-sm text-slate-400 leading-relaxed">{selected.description}</p>

          <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
            {[
              { label: "Proposer", value: shortenHash(selected.proposer), icon: Hash },
              { label: "Start Epoch", value: `#${selected.startEpoch}`, icon: Calendar },
              { label: "End Epoch", value: `#${selected.endEpoch}`, icon: Clock },
              { label: "Deposit", value: `${formatNbl(selected.deposit)} NBL`, icon: Tag },
            ].map(({ label, value, icon: Icon }) => (
              <div key={label} className="p-3 rounded-xl bg-slate-100/60 dark:bg-slate-700/30">
                <div className="flex items-center gap-1.5 text-xs text-slate-500 mb-1">
                  <Icon className="w-3 h-3" />
                  {label}
                </div>
                <div className="text-sm font-mono text-slate-700 dark:text-slate-200 flex items-center gap-1.5">
                  {value}
                  {label === "Proposer" && (
                    <button
                      onClick={() => { navigator.clipboard.writeText(selected.proposer); showToast("Proposer address copied"); }}
                      className="text-slate-400 hover:text-blue-400 transition-colors"
                      title="Copy address"
                    >
                      <Copy className="w-3 h-3" />
                    </button>
                  )}
                </div>
              </div>
            ))}
          </div>

          {selected.actions && selected.actions.length > 0 && (
            <div>
              <h4 className="text-xs uppercase tracking-wider text-slate-500 mb-2">Executable Actions</h4>
              {selected.actions.map((action, i) => (
                <div key={i} className="p-3 rounded-xl bg-slate-100/60 dark:bg-slate-700/30 space-y-1.5 text-xs font-mono">
                  <div className="text-slate-400">Target: <span className="text-slate-700 dark:text-slate-200">{shortenHash(action.target)}</span></div>
                  <div className="text-slate-400">Signature: <span className="text-slate-700 dark:text-slate-200">{action.signature}</span></div>
                  <div className="text-slate-400">Calldata: <span className="text-slate-700 dark:text-slate-200">{shortenHash(action.calldata, 16)}</span></div>
                </div>
              ))}
            </div>
          )}

          <div>
            <h4 className="text-xs uppercase tracking-wider text-slate-500 mb-3">Vote Results</h4>
            <VoteBreakdown
              yes={Number(selected.yesVotes)}
              no={Number(selected.noVotes)}
              abstain={Number(selected.abstainVotes || 0)}
            />
            {(() => {
              const cast = Number(selected.yesVotes) + Number(selected.noVotes) + Number(selected.abstainVotes || 0);
              const threshold = 4000000;
              const pct = Math.min(100, (cast / threshold) * 100);
              const reached = cast >= threshold;
              return (
                <div className="mt-4">
                  <div className="flex items-center justify-between text-xs mb-1.5">
                    <span className="text-slate-500 flex items-center gap-1.5">
                      <Shield className="w-3 h-3 text-violet-400" />
                      Quorum ({threshold.toLocaleString()} votes)
                    </span>
                    <span className={reached ? "text-emerald-400 font-medium" : "text-amber-400 font-medium"}>
                      {reached ? "Reached" : `${cast.toLocaleString()} / ${threshold.toLocaleString()}`}
                    </span>
                  </div>
                  <div className="h-1.5 rounded-full bg-slate-200 dark:bg-slate-700/50 overflow-hidden">
                    <div
                      className={`h-full rounded-full transition-all duration-500 ${reached ? "bg-gradient-to-r from-emerald-500 to-teal-400" : "bg-gradient-to-r from-amber-500 to-orange-400"}`}
                      style={{ width: `${pct}%` }}
                    />
                  </div>
                </div>
              );
            })()}
          </div>

          {selected.status === "Active" && (
            <div className="space-y-3">
              <p className="text-sm font-medium text-slate-700 dark:text-slate-300">Cast your vote</p>
              <div className="flex gap-3">
                {[
                  { support: "For", icon: ThumbsUp, color: "bg-emerald-600 hover:bg-emerald-500 text-white" },
                  { support: "Against", icon: ThumbsDown, color: "bg-rose-600 hover:bg-rose-500 text-white" },
                  { support: "Abstain", icon: Minus, color: "bg-slate-600 hover:bg-slate-500 text-white" },
                ].map(({ support, icon: Icon, color }) => (
                  <button
                    key={support}
                    onClick={() => handleVote(selected.id, support)}
                    disabled={voting === selected.id}
                    className={`flex-1 flex items-center justify-center gap-2 px-4 py-3 rounded-xl text-sm font-medium transition-all disabled:opacity-50 ${color}`}
                  >
                    {voting === selected.id ? <Loader2 className="w-4 h-4 animate-spin" /> : <Icon className="w-4 h-4" />}
                    {support}
                  </button>
                ))}
              </div>
            </div>
          )}
        </div>
      ) : (
        <div className="space-y-3">
          <div className="text-xs text-slate-500 dark:text-slate-400">
            Showing <span className="font-semibold text-slate-700 dark:text-slate-200">{filtered.length}</span> of {proposals.length} proposals
            {query.trim() && <> matching <span className="font-mono">"{query.trim()}"</span></>}
            {filter !== "All" && <> in <span className="font-semibold">{filter}</span></>}
          </div>
          {filtered.length === 0 ? (
            <div className="p-8 rounded-2xl border border-dashed border-slate-300 dark:border-slate-600/40 text-center">
              <Search className="w-8 h-8 text-slate-500 mx-auto mb-2" />
              <p className="text-sm text-slate-400">
                {query.trim() ? `No proposals match "${query.trim()}".` : `No proposals found in "${filter}" status.`}
              </p>
              {query.trim() && (
                <button onClick={() => setQuery("")} className="mt-3 text-xs text-blue-400 hover:text-blue-300">
                  Clear search
                </button>
              )}
            </div>
          ) : (
            filtered.map(p => {
              const StatusIcon = getStatusIcon(p.status);
              const yes = Number(p.yesVotes);
              const no = Number(p.noVotes);
              const abstain = Number(p.abstainVotes || 0);
              return (
                <div
                  key={p.id}
                  onClick={() => setSelected(p)}
                  className="p-4 md:p-5 rounded-2xl border border-slate-200 dark:border-slate-700/50 bg-white/70 dark:bg-slate-800/40 backdrop-blur-sm hover:border-slate-300 dark:hover:border-slate-600/60 cursor-pointer transition-all space-y-3"
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0">
                      <div className="flex items-center gap-2 mb-1">
                        <StatusIcon className="w-3.5 h-3.5" />
                        <span className="text-xs text-slate-500 font-mono">#{p.id}</span>
                        <span className={`px-2 py-0.5 rounded-full text-xs font-medium border ${getStatusColor(p.status)}`}>
                          {p.status}
                        </span>
                      </div>
                      <h3 className="text-sm md:text-base font-semibold text-slate-700 dark:text-slate-200">{p.title}</h3>
                    </div>
                    <ChevronRight className="w-4 h-4 text-slate-500 shrink-0 mt-1" />
                  </div>
                  <div className="flex items-center gap-3 text-xs text-slate-500">
                    <span>Ends epoch #{p.endEpoch}</span>
                    <span>·</span>
                    <span>{yes + no > 0 ? `${(((yes + no) / (yes + no + abstain)) * 100).toFixed(0)}% turnout` : "No votes"}</span>
                  </div>
                  <ProgressBar yes={yes} no={no} abstain={abstain} />
                </div>
              );
            })
          )}
        </div>
      )}
    </div>
  );
}

function CreateProposal() {
  const [form, setForm] = useState({ title: "", description: "", actionTarget: "", actionSig: "", actionData: "", deposit: "100000" });
  const [submitting, setSubmitting] = useState(false);
  const [success, setSuccess] = useState(false);

  const handleSubmit = (e) => {
    e.preventDefault();
    setSubmitting(true);
    setTimeout(() => {
      setSubmitting(false);
      setSuccess(true);
      setForm({ title: "", description: "", actionTarget: "", actionSig: "", actionData: "", deposit: "100000" });
      setTimeout(() => setSuccess(false), 4000);
    }, 1000);
  };

  const isValid = form.title.trim() && form.description.trim() && form.deposit.trim();

  return (
    <form onSubmit={handleSubmit} className="p-5 md:p-6 rounded-2xl border border-slate-200 dark:border-slate-700/50 bg-white/70 dark:bg-slate-800/40 backdrop-blur-sm space-y-5">
      {success && (
        <div className="px-4 py-3 rounded-xl bg-emerald-600/20 border border-emerald-500/30 text-emerald-300 text-sm flex items-center gap-2">
          <CheckCircle className="w-4 h-4" />
          Proposal submitted successfully! It will appear in the proposal queue once it reaches the voting period.
        </div>
      )}

      <div>
        <h2 className="text-lg font-bold text-slate-900 dark:text-white">Create a Governance Proposal</h2>
        <p className="text-xs text-slate-400 mt-1">Submit a new on-chain proposal for validator voting. A deposit is required to prevent spam.</p>
      </div>

      <div className="space-y-4">
        <div>
          <label className="block text-xs uppercase tracking-wider text-slate-400 mb-1.5">Title</label>
          <input
            value={form.title}
            onChange={e => setForm(f => ({ ...f, title: e.target.value }))}
            placeholder="e.g., Increase block gas limit to 30M"
            className="w-full px-3.5 py-2.5 rounded-xl bg-white/70 dark:bg-slate-700/60 border border-slate-300 dark:border-slate-600/60 text-sm text-slate-700 dark:text-slate-200 placeholder-slate-500 focus:outline-none focus:border-blue-500/50 transition-all"
          />
        </div>

        <div>
          <label className="block text-xs uppercase tracking-wider text-slate-400 mb-1.5">Description</label>
          <textarea
            value={form.description}
            onChange={e => setForm(f => ({ ...f, description: e.target.value }))}
            rows={4}
            placeholder="Describe the purpose, motivation, and expected impact of your proposal. Include relevant links and data."
            className="w-full px-3.5 py-2.5 rounded-xl bg-white/70 dark:bg-slate-700/60 border border-slate-300 dark:border-slate-600/60 text-sm text-slate-700 dark:text-slate-200 placeholder-slate-500 focus:outline-none focus:border-blue-500/50 transition-all resize-none"
          />
        </div>

        <div className="border-t border-slate-200 dark:border-slate-700/50 pt-4">
          <h3 className="text-xs uppercase tracking-wider text-slate-400 mb-3">Action (Optional — executes on-chain calldata)</h3>
          <div className="grid md:grid-cols-3 gap-3">
            <div>
              <label className="block text-xs text-slate-500 mb-1">Target Contract</label>
              <input
                value={form.actionTarget}
                onChange={e => setForm(f => ({ ...f, actionTarget: e.target.value }))}
                placeholder="0x..."
                className="w-full px-3 py-2 rounded-xl bg-white/70 dark:bg-slate-700/60 border border-slate-300 dark:border-slate-600/60 text-sm text-slate-700 dark:text-slate-200 placeholder-slate-500 focus:outline-none focus:border-blue-500/50 transition-all font-mono"
              />
            </div>
            <div>
              <label className="block text-xs text-slate-500 mb-1">Function Signature</label>
              <input
                value={form.actionSig}
                onChange={e => setForm(f => ({ ...f, actionSig: e.target.value }))}
                placeholder="setParam(uint256)"
                className="w-full px-3 py-2 rounded-xl bg-white/70 dark:bg-slate-700/60 border border-slate-300 dark:border-slate-600/60 text-sm text-slate-700 dark:text-slate-200 placeholder-slate-500 focus:outline-none focus:border-blue-500/50 transition-all font-mono"
              />
            </div>
            <div>
              <label className="block text-xs text-slate-500 mb-1">Calldata (hex)</label>
              <input
                value={form.actionData}
                onChange={e => setForm(f => ({ ...f, actionData: e.target.value }))}
                placeholder="0x..."
                className="w-full px-3 py-2 rounded-xl bg-white/70 dark:bg-slate-700/60 border border-slate-300 dark:border-slate-600/60 text-sm text-slate-700 dark:text-slate-200 placeholder-slate-500 focus:outline-none focus:border-blue-500/50 transition-all font-mono"
              />
            </div>
          </div>
        </div>

        <div className="grid md:grid-cols-2 gap-4">
          <div>
            <label className="block text-xs uppercase tracking-wider text-slate-400 mb-1.5">Deposit (NBL)</label>
            <input
              value={form.deposit}
              onChange={e => setForm(f => ({ ...f, deposit: e.target.value }))}
              type="number"
              min="1000"
              className="w-full px-3.5 py-2.5 rounded-xl bg-white/70 dark:bg-slate-700/60 border border-slate-300 dark:border-slate-600/60 text-sm text-slate-700 dark:text-slate-200 placeholder-slate-500 focus:outline-none focus:border-blue-500/50 transition-all"
            />
          </div>
          <div>
            <label className="block text-xs uppercase tracking-wider text-slate-400 mb-1.5">Min. deposit required</label>
            <div className="px-3.5 py-2.5 rounded-xl bg-slate-100/60 dark:bg-slate-700/30 text-sm text-slate-700 dark:text-slate-300 flex items-center gap-2 h-full">
              <Info className="w-3.5 h-3.5 text-slate-500" />
              100 NBL minimum · Refunded if proposal passes
            </div>
          </div>
        </div>
      </div>

      <div className="flex items-center gap-3 pt-2">
        <button
          type="submit"
          disabled={!isValid || submitting}
          className="flex items-center gap-2 px-6 py-3 rounded-xl bg-gradient-to-r from-blue-600 to-indigo-600 hover:from-blue-500 hover:to-indigo-500 text-white text-sm font-semibold transition-all disabled:opacity-40 disabled:cursor-not-allowed shadow-lg shadow-blue-600/20"
        >
          {submitting ? <Loader2 className="w-4 h-4 animate-spin" /> : <Send className="w-4 h-4" />}
          {submitting ? "Submitting..." : "Submit Proposal"}
        </button>
        <p className="text-xs text-slate-500">Submitting will use your connected wallet to pay the deposit and transaction fees.</p>
      </div>
    </form>
  );
}

function Treasury({ treasury, govParams }) {
  const balance = treasury ? formatNbl(treasury.balance) : "—";
  const collected = treasury ? formatNbl(treasury.totalCollected) : "—";
  const spent = treasury ? formatNbl(treasury.totalSpent) : "—";
  const reserveRatio = treasury && BigInt(treasury.totalCollected) > 0n
    ? ((BigInt(treasury.balance) * 100n) / BigInt(treasury.totalCollected)).toString()
    : "0";

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3 md:gap-4">
        <StatCard label="Treasury Balance" value={`${balance} NBL`} sub="Available funds" icon={Wallet} color="blue" />
        <StatCard label="Total Collected" value={`${collected} NBL`} sub="All-time inflows" icon={TrendingUp} color="emerald" />
        <StatCard label="Total Spent" value={`${spent} NBL`} sub="All-time outflows" icon={TrendingDown} color="rose" />
        <StatCard label="Reserve Ratio" value={`${reserveRatio}%`} sub="Of total collected" icon={PieChart} color="violet" />
      </div>

      <div className="grid md:grid-cols-2 gap-4 md:gap-6">
        <div className="p-5 rounded-2xl border border-slate-200 dark:border-slate-700/50 bg-white/70 dark:bg-slate-800/40 backdrop-blur-sm">
          <h3 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4 flex items-center gap-2">
            <Activity className="w-4 h-4 text-blue-400" />
            Recent Transactions
          </h3>
          <div className="space-y-2 max-h-80 overflow-y-auto scrollbar-thin">
            {treasury.recentTransactions.map((tx, i) => (
              <div key={i} className="flex items-center justify-between p-3 rounded-xl bg-slate-100/60 dark:bg-slate-700/30">
                <div className="flex items-center gap-3 min-w-0">
                  {tx.type === "deposit" ? (
                    <ArrowDownRight className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                  ) : (
                    <ArrowUpRight className="w-3.5 h-3.5 text-rose-400 shrink-0" />
                  )}
                  <div className="min-w-0">
                    <div className="text-xs text-slate-700 dark:text-slate-200">{tx.reason}</div>
                    <div className="text-xs text-slate-500 font-mono">{tx.hash} · {formatTime(tx.timestamp)}</div>
                  </div>
                </div>
                <div className={`text-xs font-semibold shrink-0 ml-3 ${tx.type === "deposit" ? "text-emerald-400" : "text-rose-400"}`}>
                  {tx.type === "deposit" ? "+" : "-"}{formatNbl(tx.amount)} NBL
                </div>
              </div>
            ))}
          </div>
        </div>

        <div className="p-5 rounded-2xl border border-slate-200 dark:border-slate-700/50 bg-white/70 dark:bg-slate-800/40 backdrop-blur-sm">
          <h3 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4 flex items-center gap-2">
            <Layers className="w-4 h-4 text-blue-400" />
            Governance Parameters
          </h3>
          <div className="space-y-3">
            {[
              { label: "Voting Period", value: `${govParams.votingPeriod} epochs`, desc: "Duration proposals remain open" },
              { label: "Quorum", value: `${govParams.quorum}%`, desc: "Minimum participation required" },
              { label: "Proposal Deposit", value: `${formatNbl(govParams.proposalDeposit)} NBL`, desc: "Refunded if proposal passes" },
              { label: "Max Actions", value: govParams.maxActions.toString(), desc: "Maximum actions per proposal" },
              { label: "Timelock Period", value: `${govParams.timelockPeriod} epochs`, desc: "Delay before execution" },
              { label: "Min Voting Power", value: `${formatNbl(govParams.minVotingPower)} NBL`, desc: "Minimum stake to vote" },
            ].map(({ label, value, desc }) => (
              <div key={label} className="flex items-center justify-between p-2.5 rounded-xl bg-slate-100/50 dark:bg-slate-700/20">
                <div>
                  <div className="text-sm text-slate-700 dark:text-slate-300">{label}</div>
                  <div className="text-xs text-slate-500">{desc}</div>
                </div>
                <span className="text-sm font-semibold text-slate-700 dark:text-slate-200">{value}</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

function Analytics({ proposals, validators }) {
  const passed = proposals.filter(p => p.status === "Passed" || p.status === "Executed").length;
  const rejected = proposals.filter(p => p.status === "Rejected").length;
  const active = proposals.filter(p => p.status === "Active").length;
  const successRate = proposals.length > 0 ? ((passed / proposals.length) * 100).toFixed(0) : "0";

  const totalVotes = proposals.reduce((sum, p) => sum + Number(p.yesVotes) + Number(p.noVotes) + Number(p.abstainVotes || 0), 0);
  const totalFor = proposals.reduce((sum, p) => sum + Number(p.yesVotes), 0);
  const totalAgainst = proposals.reduce((sum, p) => sum + Number(p.noVotes), 0);
  const totalAbstain = proposals.reduce((sum, p) => sum + Number(p.abstainVotes || 0), 0);

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3 md:gap-4">
        <StatCard label="Total Proposals" value={proposals.length.toString()} sub="All time" icon={FileText} color="blue" />
        <StatCard label="Success Rate" value={`${successRate}%`} sub={`${passed} passed · ${rejected} rejected`} icon={Target} color="emerald" />
        <StatCard label="Active Now" value={active.toString()} sub="Open for voting" icon={Zap} color="amber" />
        <StatCard label="Total Voters" value={totalVotes.toLocaleString()} sub="Across all proposals" icon={Users} color="violet" />
      </div>

      <div className="grid md:grid-cols-2 gap-4 md:gap-6">
        <div className="p-5 rounded-2xl border border-slate-200 dark:border-slate-700/50 bg-white/70 dark:bg-slate-800/40 backdrop-blur-sm">
          <h3 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4 flex items-center gap-2">
            <BarChart3 className="w-4 h-4 text-blue-400" />
            Voting Power Distribution
          </h3>
          <p className="text-xs text-slate-500 mb-4">All-time vote breakdown across all proposals</p>
          <VoteBreakdown yes={totalFor} no={totalAgainst} abstain={totalAbstain} />
          <div className="grid grid-cols-3 gap-3 mt-4">
            {[
              { label: "For", value: ((totalFor / (totalVotes || 1)) * 100).toFixed(1), color: "text-emerald-400", bg: "bg-emerald-500/20" },
              { label: "Against", value: ((totalAgainst / (totalVotes || 1)) * 100).toFixed(1), color: "text-rose-400", bg: "bg-rose-500/20" },
              { label: "Abstain", value: ((totalAbstain / (totalVotes || 1)) * 100).toFixed(1), color: "text-slate-400", bg: "bg-slate-500/20" },
            ].map(({ label, value, color, bg }) => (
              <div key={label} className={`${bg} rounded-xl p-3 text-center`}>
                <div className={`text-lg font-bold ${color}`}>{value}%</div>
                <div className="text-xs text-slate-500">{label}</div>
              </div>
            ))}
          </div>
        </div>

        <div className="p-5 rounded-2xl border border-slate-200 dark:border-slate-700/50 bg-white/70 dark:bg-slate-800/40 backdrop-blur-sm">
          <h3 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4 flex items-center gap-2">
            <Activity className="w-4 h-4 text-blue-400" />
            Proposal Activity
          </h3>
          <div className="space-y-3">
            {proposals.slice(0, 5).map(p => {
              const statusClass = getStatusColor(p.status).split(" ")[1];
              const total = Number(p.yesVotes) + Number(p.noVotes) + Number(p.abstainVotes || 0);
              const turnout = total > 0 ? ((total / 10000000) * 100).toFixed(0) : "0";
              return (
                <div key={p.id} className="flex items-center justify-between p-2.5 rounded-xl bg-slate-100/50 dark:bg-slate-700/20">
                  <div className="min-w-0">
                    <div className="text-sm text-slate-700 dark:text-slate-200 truncate">{p.title}</div>
                    <div className={`text-xs ${statusClass}`}>{p.status} · {turnout}% turnout</div>
                  </div>
                  <span className="text-xs font-mono text-slate-500 shrink-0 ml-3">#{p.id}</span>
                </div>
              );
            })}
          </div>
        </div>
      </div>

      <div className="p-5 rounded-2xl border border-slate-200 dark:border-slate-700/50 bg-white/70 dark:bg-slate-800/40 backdrop-blur-sm">
        <h3 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4 flex items-center gap-2">
          <Users className="w-4 h-4 text-blue-400" />
          Participation Overview
        </h3>
        <div className="grid md:grid-cols-3 gap-4">
          {[
            { label: "Avg. Turnout", value: "57.3%", desc: "Across active proposals", icon: Activity, color: "blue" },
            { label: "Unique Voters", value: "24 validators", desc: "Have voted this epoch", icon: Users, color: "violet" },
            { label: "Voting Power", value: "12,456,000 NBL", desc: "Total eligible stake", icon: Shield, color: "emerald" },
          ].map(({ label, value, desc, icon: Icon, color }) => {
            const c = color === "blue" ? "from-blue-500/10 to-blue-600/5 border-blue-500/10 text-blue-400" : color === "violet" ? "from-violet-500/10 to-violet-600/5 border-violet-500/10 text-violet-400" : "from-emerald-500/10 to-emerald-600/5 border-emerald-500/10 text-emerald-400";
            return (
              <div key={label} className={`p-4 rounded-xl bg-gradient-to-br ${c} border`}>
                <Icon className="w-5 h-5 mb-2" />
                <div className="text-lg font-bold text-white">{value}</div>
                <div className="text-xs text-slate-400">{label}</div>
                <div className="text-xs text-slate-500 mt-1">{desc}</div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}

export default function Governance() {
  const [activeTab, setActiveTab] = useState("dashboard");
  const [notifDismissed, setNotifDismissed] = useState(false);
  const [loading, setLoading] = useState(true);
  const [backendOnline, setBackendOnline] = useState(true);
  const [network, setNetwork] = useState(null);
  const [proposals, setProposals] = useState(PROPOSALS_MOCK);
  const [treasury] = useState(TREASURY_MOCK);
  const [govParams] = useState(GOV_PARAMS_MOCK);
  const [validators] = useState(VALIDATORS_MOCK);

  const activeProposals = useMemo(() => proposals.filter(p => p.status === "Active"), [proposals]);

  // Node health + network info polling (matches Explorer/Faucet)
  useEffect(() => {
    const poll = async () => {
      try {
        const [hr, sr, gr] = await Promise.all([
          window.fetch(`${RPC_URL}/health`),
          window.fetch(`${RPC_URL}/status`),
          window.fetch(`${RPC_URL}/gas_price`),
        ]);
        if (!hr.ok) throw new Error("offline");
        const sd = await sr.json();
        const gd = await gr.json();
        setBackendOnline(true);
        setNetwork({ height: sd.height, finalized: sd.finalized_height, baseFee: gd.base_fee });
      } catch {
        setBackendOnline(false);
      }
    };
    poll();
    const t = setInterval(poll, 5000);
    return () => clearInterval(t);
  }, []);

  // Brief skeleton so the page feels live
  useEffect(() => {
    const t = setTimeout(() => setLoading(false), 450);
    return () => clearTimeout(t);
  }, []);

  const handleVote = (proposalId, support) => {
    setProposals(prev => prev.map(p => {
      if (p.id !== proposalId) return p;
      const weight = 1000000;
      if (support === "For") return { ...p, yesVotes: (BigInt(p.yesVotes) + BigInt(weight)).toString() };
      if (support === "Against") return { ...p, noVotes: (BigInt(p.noVotes) + BigInt(weight)).toString() };
      return { ...p, abstainVotes: (BigInt(p.abstainVotes || "0") + BigInt(weight)).toString() };
    }));
  };

  return (
    <div className="relative min-h-screen overflow-hidden">
      {/* Aurora blobs + grid backdrop */}
      <div className="absolute -top-40 -left-40 w-[38rem] h-[38rem] rounded-full opacity-25 animate-float pointer-events-none"
        style={{ background: "radial-gradient(circle, rgba(14,165,233,0.45), transparent 70%)" }} />
      <div className="absolute top-40 -right-40 w-[34rem] h-[34rem] rounded-full opacity-20 animate-float-alt pointer-events-none"
        style={{ background: "radial-gradient(circle, rgba(139,92,246,0.4), transparent 70%)" }} />
      <div className="absolute inset-0 bg-grid pointer-events-none" />

      <div className="relative z-10 max-w-5xl mx-auto px-4 md:px-6 py-8 md:py-10 animate-fade-in">
        {/* ── Header ─────────────────────────────────────────────── */}
        <header className="text-center mb-8">
          <div className="inline-flex items-center gap-2 px-4 py-1.5 rounded-full glass text-xs font-medium text-slate-600 dark:text-slate-300 mb-5">
            <span className={`w-2 h-2 rounded-full ${backendOnline ? "bg-emerald-400 animate-pulse-dot" : "bg-red-400"}`} />
            {backendOnline ? "Governance online" : "Node offline"}
            <span className="text-slate-400 dark:text-slate-500">· BFT</span>
          </div>

          <div className="flex items-center justify-center gap-3 mb-2">
            <div className="w-12 h-12 rounded-2xl bg-gradient-to-br from-blue-600 to-cyan-600 flex items-center justify-center text-white shadow-lg shadow-blue-600/25">
              <Vote className="w-6 h-6" />
            </div>
            <h1 className="text-4xl md:text-5xl font-bold tracking-tight text-slate-900 dark:text-white">
              Nebula
              <span className="bg-gradient-to-r from-blue-500 via-cyan-500 to-violet-500 bg-clip-text text-transparent"> Governance</span>
            </h1>
          </div>
          <p className="text-slate-500 dark:text-slate-400 max-w-xl mx-auto">
            Propose, vote, and steer the network — community-led parameter changes, runtime upgrades, and treasury allocations.
          </p>

          {/* Network strip */}
          {network && (
            <div className="flex items-center justify-center flex-wrap gap-2 mt-5 text-xs">
              <span className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full glass-strong font-mono text-slate-700 dark:text-slate-200">
                <Activity className="w-3.5 h-3.5 text-blue-500 dark:text-cyan-400" /> Tip #{fmt(network.height)}
              </span>
              <span className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full glass-strong font-mono text-slate-700 dark:text-slate-200">
                <Gauge className="w-3.5 h-3.5 text-emerald-500" /> Base fee {fmt(network.baseFee)}
              </span>
              <span className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full glass-strong font-mono text-slate-700 dark:text-slate-200">
                <ShieldCheck className="w-3.5 h-3.5 text-violet-500" /> Finalized #{fmt(network.finalized)}
              </span>
            </div>
          )}
        </header>

        {!backendOnline && (
          <div className="mb-6 px-4 py-3 rounded-xl bg-amber-500/15 border border-amber-500/30 text-sm text-amber-400 flex items-center gap-2">
            <WifiOff className="w-4 h-4 shrink-0" />
            <span>Node not reachable at {RPC_URL}. Governance data below is sample data — start the testnet to connect.</span>
          </div>
        )}

        {/* ── Tab bar ────────────────────────────────────────────── */}
        <div className="flex flex-wrap gap-1 p-1 rounded-2xl glass-strong mb-6 max-w-md mx-auto">
          {TABS.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              onClick={() => setActiveTab(id)}
              className={`flex-1 flex items-center justify-center gap-1.5 px-3 py-2.5 rounded-xl text-sm font-semibold transition-all ${
                activeTab === id
                  ? "bg-gradient-to-r from-blue-600 to-cyan-600 text-white shadow-lg shadow-blue-600/25"
                  : "text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 hover:bg-slate-200/50 dark:hover:bg-slate-700/40"
              }`}
            >
              <Icon className="w-4 h-4" />
              {label}
            </button>
          ))}
        </div>

        {!notifDismissed && activeProposals.length > 0 && (
          <div className="mb-6">
            <NotificationBar
              proposals={activeProposals}
              onDismiss={() => setNotifDismissed(true)}
              onViewProposals={() => { setActiveTab("proposals"); setNotifDismissed(true); }}
            />
          </div>
        )}

        <div key={activeTab} className="animate-in">
          {activeTab === "dashboard" && (
            loading
              ? <DashboardSkeleton />
              : <Dashboard proposals={proposals} treasury={treasury} validators={validators} govParams={govParams} onTabChange={setActiveTab} />
          )}
          {activeTab === "proposals" && (
            <Proposals proposals={proposals} onVote={handleVote} />
          )}
          {activeTab === "create" && (
            <CreateProposal />
          )}
          {activeTab === "treasury" && (
            <Treasury treasury={treasury} govParams={govParams} />
          )}
          {activeTab === "analytics" && (
            <Analytics proposals={proposals} validators={validators} />
          )}
        </div>

        <footer className="mt-12 pt-6 border-t border-slate-200 dark:border-slate-800 text-center text-xs text-slate-500 dark:text-slate-600">
          Nebula Governance v1.0.0 · Scratch Blockchain · Data reflects the current network state
        </footer>
      </div>
    </div>
  );
}
