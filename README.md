# Solana Native Contracts Practice

This workspace contains 5 Solana native contracts for practice:

## Contracts

### 1. Escrow (`/escrow`)
**Purpose**: Implement a secure escrow system for token exchanges
**Key Features to Implement**:
- Initialize escrow with tokens from initializer
- Allow taker to exchange tokens
- Cancel escrow and return tokens
- Handle different token types (SPL tokens)

**Common Instructions**:
- `InitializeEscrow` - Create escrow account and transfer tokens
- `Exchange` - Complete the trade
- `Cancel` - Cancel escrow and return tokens

### 2. Staking (`/staking`)
**Purpose**: Implement a token staking mechanism with rewards
**Key Features to Implement**:
- Stake tokens for rewards
- Unstake tokens with penalty periods
- Calculate and distribute rewards
- Handle multiple stakers

**Common Instructions**:
- `InitializeStakePool` - Create staking pool
- `Stake` - Stake tokens
- `Unstake` - Unstake tokens
- `ClaimRewards` - Claim accumulated rewards

### 3. Vault (`/vault`)
**Purpose**: Implement a secure multi-signature vault
**Key Features to Implement**:
- Multi-signature approval system
- Secure token storage
- Withdrawal mechanisms
- Permission management

**Common Instructions**:
- `InitializeVault` - Create vault
- `Deposit` - Deposit tokens
- `ProposeWithdrawal` - Propose withdrawal
- `ApproveWithdrawal` - Approve withdrawal
- `ExecuteWithdrawal` - Execute approved withdrawal

### 4. Marketplace (`/marketplace`)
**Purpose**: Implement an NFT/token marketplace
**Key Features to Implement**:
- List items for sale
- Buy items with various payment methods
- Handle marketplace fees
- Offer/bid system

**Common Instructions**:
- `InitializeMarketplace` - Create marketplace
- `List` - List item for sale
- `Buy` - Purchase listed item
- `Delist` - Remove listing
- `MakeOffer` - Make offer on item
- `AcceptOffer` - Accept offer

### 5. Governance (`/governance`)
**Purpose**: Implement a DAO governance system
**Key Features to Implement**:
- Create proposals
- Vote on proposals
- Execute approved proposals
- Manage voting power

**Common Instructions**:
- `InitializeRealm` - Create governance realm
- `CreateProposal` - Create new proposal
- `Vote` - Vote on proposal
- `ExecuteProposal` - Execute approved proposal
- `UpdateVotingPower` - Update user voting power

## Development Setup

### Prerequisites
- Rust installed (https://rustup.rs/)
- Solana CLI tools (https://docs.solana.com/cli/install-solana-cli-tools)

### Building
```bash
# Build all contracts
cargo build

# Build specific contract
cargo build -p escrow
cargo build -p staking
cargo build -p vault
cargo build -p marketplace
cargo build -p governance

# Build for deployment
cargo build --release
```

### Testing
```bash
# Run tests for all contracts
cargo test

# Run tests for specific contract
cargo test -p escrow
```

### Deployment
```bash
# Deploy to local validator
solana program deploy target/deploy/escrow.so

# Deploy to devnet
solana program deploy target/deploy/escrow.so --url devnet
```

## Project Structure

Each contract follows this structure:
```
contract_name/
├── src/
│   ├── lib.rs          # Entry point and exports
│   ├── error.rs        # Error definitions
│   ├── instruction.rs  # Instruction definitions
│   ├── processor.rs    # Business logic
│   └── state.rs        # Account state definitions
└── Cargo.toml         # Dependencies and configuration
```

## Key Dependencies

- `solana-program`: Core Solana program library
- `borsh`: Serialization/deserialization
- `thiserror`: Error handling
- `spl-token`: SPL token program interface
- `spl-associated-token-account`: Associated token account utilities

## Practice Tips

1. **Start with Error Types**: Define your error types first in `error.rs`
2. **Design State Structure**: Define your account structures in `state.rs`
3. **Define Instructions**: Create your instruction enum in `instruction.rs`
4. **Implement Processor**: Write the business logic in `processor.rs`
5. **Test Thoroughly**: Write comprehensive tests for each instruction

## Common Patterns

- **PDA (Program Derived Address)**: Used for deterministic account addresses
- **Cross-Program Invocation (CPI)**: Calling other programs (like SPL Token)
- **Account Validation**: Always validate account ownership and data
- **Rent Exemption**: Ensure accounts are rent-exempt
- **Security Checks**: Validate signers, account relationships, and data integrity

## Resources

- [Solana Program Library](https://spl.solana.com/)
- [Solana Developer Documentation](https://docs.solana.com/developing/programming-model/overview)
- [Anchor Book](https://book.anchor-lang.com/) - For reference on patterns
- [Solana Cookbook](https://solanacookbook.com/) - Common recipes and patterns

Happy coding! 🚀
