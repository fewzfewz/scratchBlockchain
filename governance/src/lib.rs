use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: u64,
    pub description: String,
    pub yes_votes: u64,
    pub no_votes: u64,
    pub passed: bool,
    pub executed: bool,
}

pub struct Governance {
    pub proposals: HashMap<u64, Proposal>,
    pub next_proposal_id: u64,
}

impl Governance {
    pub fn new() -> Self {
        Self {
            proposals: HashMap::new(),
            next_proposal_id: 0,
        }
    }

    pub fn create_proposal(&mut self, description: String) -> u64 {
        let id = self.next_proposal_id;
        let desc_clone = description.clone();
        let proposal = Proposal {
            id,
            description,
            yes_votes: 0,
            no_votes: 0,
            passed: false,
            executed: false,
        };
        self.proposals.insert(id, proposal);
        self.next_proposal_id += 1;
        println!("Proposal {} created: {}", id, desc_clone);
        id
    }

    pub fn vote(&mut self, proposal_id: u64, vote_yes: bool) -> bool {
        if let Some(proposal) = self.proposals.get_mut(&proposal_id) {
            if proposal.executed {
                return false; // Cannot vote on executed proposals
            }
            if vote_yes {
                proposal.yes_votes += 1;
            } else {
                proposal.no_votes += 1;
            }
            println!(
                "Voted {} on proposal {}",
                if vote_yes { "YES" } else { "NO" },
                proposal_id
            );
            true
        } else {
            false
        }
    }

    pub fn tally_votes(&mut self, proposal_id: u64) -> bool {
        if let Some(proposal) = self.proposals.get_mut(&proposal_id) {
            if proposal.yes_votes > proposal.no_votes {
                proposal.passed = true;
                println!("Proposal {} passed!", proposal_id);
                true
            } else {
                proposal.passed = false;
                println!("Proposal {} rejected.", proposal_id);
                false
            }
        } else {
            false
        }
    }

    pub fn execute_proposal(&mut self, proposal_id: u64) -> bool {
        if let Some(proposal) = self.proposals.get_mut(&proposal_id) {
            if proposal.passed && !proposal.executed {
                proposal.executed = true;
                println!(
                    "Executing proposal {}: {}",
                    proposal_id, proposal.description
                );
                // In a real system, this would trigger a code upgrade or parameter change
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}

pub fn init() {
    println!("Governance initialized (use Governance::new)");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_proposal() {
        let mut gov = Governance::new();
        let id = gov.create_proposal("Test proposal".to_string());
        assert_eq!(id, 0);
        assert_eq!(gov.proposals.len(), 1);
    }

    #[test]
    fn test_voting() {
        let mut gov = Governance::new();
        let id = gov.create_proposal("Test proposal".to_string());

        assert!(gov.vote(id, true));
        assert!(gov.vote(id, true));
        assert!(gov.vote(id, false));

        let proposal = gov.proposals.get(&id).unwrap();
        assert_eq!(proposal.yes_votes, 2);
        assert_eq!(proposal.no_votes, 1);
    }

    #[test]
    fn test_proposal_passes() {
        let mut gov = Governance::new();
        let id = gov.create_proposal("Test proposal".to_string());

        gov.vote(id, true);
        gov.vote(id, true);
        gov.vote(id, false);

        assert!(gov.tally_votes(id));
        let proposal = gov.proposals.get(&id).unwrap();
        assert!(proposal.passed);
    }

    #[test]
    fn test_proposal_execution() {
        let mut gov = Governance::new();
        let id = gov.create_proposal("Test proposal".to_string());

        gov.vote(id, true);
        gov.vote(id, true);
        gov.tally_votes(id);

        assert!(gov.execute_proposal(id));
        let proposal = gov.proposals.get(&id).unwrap();
        assert!(proposal.executed);

        // Cannot execute twice
        assert!(!gov.execute_proposal(id));
    }
}
