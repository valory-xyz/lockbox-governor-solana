# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Lockbox Governor is a Solana program (Anchor framework) that enables cross-chain governance of OLAS and SOL token accounts via Wormhole bridge messages. A Timelock contract on another chain sends Wormhole VAA messages that this program verifies and executes.

Program ID: `DWDGo2UkBUFZ3VitBfWRBMvRnHr7E2DSh57NK27xMYaB`

## Required Environment

- anchor-cli 0.29.0
- solana-cli 1.18.1
- rustc 1.74.1

Run `setup-env.sh` to install the correct versions.

## Build & Test Commands

```bash
# Install JS dependencies
yarn

# Build the Solana program
anchor build

# Run Rust unit tests (config/received size checks)
cargo test

# Run a local validator (in a separate terminal) — loads forked accounts and the deployed program
./validator.sh

# Set env vars for tests
export ANCHOR_PROVIDER_URL=http://127.0.0.1:8899
export ANCHOR_WALLET=artifacts/id.json

# Run a single integration test (restart validator between tests)
solana airdrop 10000 9fit3w7t6FHATDaZWotpWqN7NpqgL3Lm1hqUop4hAy8h --url localhost && npx ts-node tests/lockbox_governor_transfer_sol.ts

# Lint
yarn lint        # check
yarn lint:fix    # auto-fix
```

Integration tests are standalone ts-node scripts in `tests/`, not a test framework suite. Each requires a fresh validator restart and SOL airdrop before running. The `set_upgrade_authority` and `upgrade_program` tests require semi-manual steps documented in the test files.

## Architecture

### Program Instructions (5 total, all gated by Wormhole VAA verification)

1. **initialize** — Creates the Config PDA; sets emitter chain ID and Timelock address
2. **transfer** — Transfers a specified amount of SOL or OLAS tokens between ATAs
3. **transfer_all** — Transfers entire SOL and OLAS balances to destination ATAs
4. **transfer_token_accounts** — Changes ownership of SOL and OLAS token accounts to a new authority
5. **set_program_upgrade_authority** — Changes a program's upgrade authority
6. **upgrade_program** — Upgrades a program from a buffer account

### Key Accounts

- **Config** (PDA, seed: `b"config"`) — Stores Wormhole addresses, emitter chain/address, bump, and cumulative transfer totals. Acts as the signing authority (via PDA seeds) for token transfers and program upgrades.
- **Received** (PDA, seed: `b"received"` + chain + sequence) — Records processed VAA hashes to prevent replay attacks. One per Wormhole message.

### Wormhole Message Payloads

Each instruction has a corresponding message type in `message.rs` with a unique payload selector byte (0-4). Messages are deserialized from Wormhole VAA data using `wormhole_io::Readable`/`Writeable` traits wrapped in Anchor's serialization.

### Security Model

- All mutating instructions (except `initialize`) require a verified Wormhole VAA from the registered foreign emitter (Timelock)
- Config PDA constraints verify `emitter_address` and `emitter_chain` match the registered foreign emitter
- Source token accounts must be owned by the Config PDA
- The Received account's PDA derivation (chain + sequence) prevents message replay
- Only SOL (`So11111111111111111111111111111111111111112`) and OLAS (`Ez3nzG9ofodYCvEmw73XhQ87LWNYVRM2s7diB5tBZPyM`) token mints are accepted

### Feature Flags

The program compiles with network-specific Wormhole SDK features:
- `testnet` (default) — Solana devnet Wormhole
- `mainnet` — Mainnet Wormhole
- `devnet` — Tilt devnet Wormhole
