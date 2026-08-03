import { Address } from "./client";

export type ProposalStatus = "Active" | "Pending" | "Passed" | "Rejected" | "Executed" | "Failed";

export interface GovProposal {
  id: number;
  title: string;
  description: string;
  proposer: Address;
  status: ProposalStatus;
  yesVotes: string;
  noVotes: string;
  abstainVotes?: string;
  startEpoch: number;
  endEpoch: number;
  deposit: string;
  executed?: boolean;
  executionData?: string;
  actions?: GovAction[];
  /** Voter address → choice (from node GET /proposal/{id}) */
  voters?: Record<string, string>;
}

export interface GovAction {
  target: Address;
  value: string;
  signature: string;
  calldata: string;
}

export interface GovVote {
  proposalId: number;
  voter: Address;
  support: "For" | "Against" | "Abstain";
  weight: string;
  reason?: string;
  timestamp: number;
}

export interface TreasuryInfo {
  balance: string;
  totalCollected: string;
  totalSpent: string;
  recentTransactions: TreasuryTransaction[];
}

export interface TreasuryTransaction {
  hash: string;
  type: "deposit" | "spend" | "transfer";
  amount: string;
  from: Address;
  to: Address;
  reason: string;
  timestamp: number;
  height: number;
}

export interface GovParams {
  votingPeriod: number;
  quorum: number;
  proposalDeposit: string;
  maxActions: number;
  timelockPeriod: number;
  minVotingPower: string;
}

export interface DelegationInfo {
  delegator: Address;
  validator: Address;
  amount: string;
  rewards: string;
  height: number;
}

export interface ValidatorInfo {
  address: Address;
  publicKey: string;
  stake: string;
  commissionRate: number;
  isActive: boolean;
  blocksProduced: number;
  blocksMissed: number;
  delegatorCount: number;
  totalDelegated: string;
}

export interface CreateProposalRequest {
  title: string;
  description: string;
  actions: GovAction[];
  deposit: string;
}

export interface VoteRequest {
  proposalId: number;
  support: "For" | "Against" | "Abstain";
  voter: Address;
  reason?: string;
}

export interface DelegateRequest {
  delegator: Address;
  validator: Address;
  amount: string;
}

export interface GovStats {
  totalProposals: number;
  activeProposals: number;
  totalVotes: number;
  totalDelegators: number;
  totalStaked: string;
  votingPower: string;
  inflationRate: number;
  activeValidators: number;
}
