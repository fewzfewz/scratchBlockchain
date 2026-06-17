// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract DAO {
    struct Proposal {
        uint256 id;
        address proposer;
        string description;
        uint256 forVotes;
        uint256 againstVotes;
        uint256 startBlock;
        uint256 endBlock;
        bool executed;
        bytes calldata_;
    }

    address public governanceToken;
    uint256 public proposalCount;
    uint256 public votingPeriod = 100;
    uint256 public quorum = 1000e18;
    uint256 public proposalThreshold = 100e18;

    mapping(uint256 => Proposal) public proposals;
    mapping(uint256 => mapping(address => bool)) public hasVoted;

    event ProposalCreated(uint256 indexed id, address proposer, string description);
    event VoteCast(uint256 indexed proposalId, address voter, bool support, uint256 weight);
    event ProposalExecuted(uint256 indexed id);

    modifier onlyTokenHolder() {
        require(IERC20(governanceToken).balanceOf(msg.sender) >= proposalThreshold, "DAO: insufficient tokens");
        _;
    }

    constructor(address _governanceToken) {
        governanceToken = _governanceToken;
    }

    function propose(string calldata description, bytes calldata data) external onlyTokenHolder {
        proposalCount++;
        proposals[proposalCount] = Proposal({
            id: proposalCount,
            proposer: msg.sender,
            description: description,
            forVotes: 0,
            againstVotes: 0,
            startBlock: block.number,
            endBlock: block.number + votingPeriod,
            executed: false,
            calldata_: data
        });
        emit ProposalCreated(proposalCount, msg.sender, description);
    }

    function castVote(uint256 proposalId, bool support) external {
        Proposal storage prop = proposals[proposalId];
        require(block.number >= prop.startBlock && block.number <= prop.endBlock, "DAO: voting not active");
        require(!hasVoted[proposalId][msg.sender], "DAO: already voted");

        uint256 weight = IERC20(governanceToken).balanceOf(msg.sender);
        require(weight > 0, "DAO: no voting power");

        hasVoted[proposalId][msg.sender] = true;

        if (support) {
            prop.forVotes += weight;
        } else {
            prop.againstVotes += weight;
        }

        emit VoteCast(proposalId, msg.sender, support, weight);
    }

    function execute(uint256 proposalId) external {
        Proposal storage prop = proposals[proposalId];
        require(block.number > prop.endBlock, "DAO: voting still active");
        require(!prop.executed, "DAO: already executed");
        require(prop.forVotes + prop.againstVotes >= quorum, "DAO: quorum not met");
        require(prop.forVotes > prop.againstVotes, "DAO: proposal defeated");

        prop.executed = true;
        (bool success,) = address(this).call(prop.calldata_);
        require(success, "DAO: execution failed");
        emit ProposalExecuted(proposalId);
    }
}

interface IERC20 {
    function balanceOf(address account) external view returns (uint256);
    function transfer(address to, uint256 amount) external returns (bool);
}
