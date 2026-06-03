# Internal audit 2 — `valory-xyz/lockbox-governor-solana`

**Date:** 2026-06-03
**Auditor:** audit-claude, 77ph
**Scope baseline:** HEAD `735802d` (2026-05-25 «Update LICENSE») on branch `main`
**Code under audit:** `programs/fee_collector/` — 2 `.rs` files, **242 LoC** (lib.rs 209 + state.rs 33)
**Toolchain (per `setup-env.sh` / README):** rustc 1.62.0, Solana 1.14.29, Anchor 0.26.0
**Mainnet program ID:** `DWDGo2UkBUFZ3VitBfWRBMvRnHr7E2DSh57NK27xMYaB` — **NOT DEPLOYED ON MAINNET** (mainnet RPC `getAccountInfo` returns null; verified 2026-06-03)
**Approach:** independent first-pass review of `programs/fee_collector/` current state. Treats the existing `audits/internal/` (2024-06 on branch `bridging4`) as historical context only; audit at main-branch HEAD.
**Methodology:** exhaustive Anchor + Solana + DeFi cross-domain pattern checklist; adversarial sweep on what is NOT in the documented intent; verify-claimed-equivalences against actual code per VCE discipline. Security-only audit scope.

---

## 1. Verdict

**FAIL — 3 Critical-class design defects in admin instructions; do NOT deploy this code to mainnet as-is.**

| Severity | Count | Items |
|---|---|---|
| Critical (design; dormant in current production) | 3 | F-1 (`transfer` has no access control), F-2 (`transfer_token_account` has no access control + Unchecked destination), F-3 (`change_upgrade_authority` has no access control + three Unchecked accounts with «CHECK: Check later» markers) |
| High | 0 | — |
| Medium | 0 | — |
| Low | 0 | — |
| Info | 8 | F-4..F-11 — dead state fields, unused event, unused error, unused imports, stale toolchain, unused constants, no-auth init, on-chain verification recommendation |

**The three Critical-class findings have a specific safety caveat.** They are dormant in mainnet today because:

1. The program ID `DWDGo2UkBUFZ3VitBfWRBMvRnHr7E2DSh57NK27xMYaB` is **not currently deployed on Solana mainnet** (verified by RPC `getAccountInfo` lookup, 2026-06-03).
2. The mainnet `lockbox-solana` program (`1BoXeb8hobfLCHNsyCoG1jpEv41ez4w4eDrJ48N1jY3`) has its actual upgrade authority set to `7mQQw6qCj66EtfarNKUfWGUD1pX7SSjHbVMpdJc1roK2` (verified by RPC `getAccountInfo` lookup on the ProgramData PDA `4db7h3HiTf8DHe5WYLbkEFiG7J7R5xJKqvHvpWuz8KXc`, 2026-06-03) — NOT this fee_collector program's PDA.

So no real user funds are at risk today. **However**, the code defects are real, and if anyone deploys this code as-is and routes any value through it (SPL token accounts owned by its PDA, or upgrade authority of any upgradeable program assigned to its PDA), those assets become drainable / takeable by any anonymous signer. The audit verdict is FAIL on the code; whether the project has currently sidestepped the risk by not deploying is operational context, not a code-quality acquittal.

**The README's claim «Timelock governed OLAS and SOL Fee Collector on Solana» is not backed by on-chain enforcement.** There is no timelock check, no admin check, no multisig check, no governance check anywhere in the code. The three administrative instructions are open to any signer. Per VCE (verify-claimed-equivalences-against-actual-code) discipline, the README's «timelock governed» claim was checked against the actual code and the equivalence does not hold. The architectural-debt analysis in **§2.5 below** explains what the dev was almost certainly building toward — the Olas L1-Governance → Timelock → bridge → L2-Executor pattern used on every other Olas-governed chain — and what would be needed to complete it on the Solana side.

The other 8 Info findings are non-critical hygiene / dead-code / toolchain issues, same family as the prior two Solana audits this week.

---

## 2. Project intent & architectural review

### What this is supposed to be

`lockbox-governor-solana` (program crate name `fee_collector`) is intended to be the **governance layer for the `lockbox-solana` protocol-owned-liquidity contract.** Three architectural responsibilities are visible in the code intent:

1. **Hold fees collected from `lockbox-solana` withdrawals.** Each withdraw on `lockbox-solana` routes LP fees to `fee_collector_token_owner_account_a/b` (per the lockbox-solana audit §3, fee_collector accounts are owned by the lockbox-solana initializer; the design intent is to migrate those account-owners to this PDA via `transfer_token_account`).
2. **Hold upgrade authority over `lockbox-solana`.** The test file `tests/fee_collector_change_upgrade_authority.ts` explicitly demonstrates calling `change_upgrade_authority` against a lockbox program (line 35: `const lockbox = new anchor.web3.PublicKey("1okwt4nGbpr82kkr6t1767sAenfeZBxUyzJAAaumZRG");` — a now-superseded lockbox program ID; the current mainnet `1BoXeb8...jY3` was the production successor).
3. **Be gated by a timelock / DAO governance mechanism**, per the README's «Timelock governed» claim.

Conceptually this is a sound governance shape: a separate program holds the privileged authorities, and access is mediated by an off-chain or on-chain governance mechanism. The EVM equivalent in the Olas ecosystem is the Timelock + Governor contracts pattern (per OpenZeppelin Governance).

### Is the architectural design sound?

**The concept is sound. The implementation is not.**

The three administrative instructions (`transfer`, `transfer_token_account`, `change_upgrade_authority`) ARE the right shape — they're the three operations a fee-collector + upgrade-authority holder needs. The PROBLEM is that the gate mechanism is **completely absent**:

- There is no `governance_authority` field stored in the `FeeCollector` state.
- There is no `admin` field.
- There is no `multisig_account` field.
- There is no timelock-program-CPI check.
- The `signer: Signer<'info>` in each privileged instruction's account struct requires only that the caller BE a signer — not that they be any specific authority.

The dev clearly knew validation was missing — three `/// CHECK: Check later` comments on UncheckedAccount fields in the `ChangeUpgradeAuthorityFeeCollector` and `TransferTokenAccountFeeCollector` structs are explicit markers that auditor-level validation was deferred. «Check later» was never honored.

### Are the developers on the right path?

**Yes for what's implemented; no for what's missing.** What's right:

- **Single global PDA seeded by `b"fee_collector"`** — correct Solana idiom for singleton program governance state.
- **PDA-signed CPI calls** to SPL Token (`token::transfer`, `set_authority`) and BPF Upgradeable Loader (`set_upgrade_authority`) — uses Anchor's `CpiContext::new_with_signer` + `invoke_signed` correctly with `&[&collector.seeds()]` for PDA signature.
- **Bump persistence** in `FeeCollector.bump` — enables deterministic seed derivation without re-`find_program_address` calls.
- **`overflow-checks = true`** in `[profile.release]` (both root and program Cargo.toml).
- **Hardcoded SOL and OLAS constants** at lib.rs:21-23 — declared intent (even if not enforced — F-11).
- **`seeds = true`** in Anchor.toml — automatic PDA seed verification.

What's missing — the gate mechanism:

- No `governance_authority` / `admin` field in state.
- No constraint on `signer` in privileged account structs (just `Signer<'info>` with no `address = ...` check).
- No timelock CPI check.
- The three «CHECK: Check later» markers indicate explicit validation that was never implemented.

This pattern — admin functions written, gate forgotten — is the most common Solana governance bug class. The fix is small (~5-10 lines per instruction):

```rust
// In FeeCollector state:
pub governance_authority: Pubkey,  // set at init from a hardcoded admin or
                                    // from the initializer signer

// In TransferFeeCollector / TransferTokenAccountFeeCollector /
//    ChangeUpgradeAuthorityFeeCollector account structs:
#[account(constraint = signer.key() == &collector.governance_authority)]
pub signer: Signer<'info>,

// Or, alternatively, hardcode a multisig PDA constant:
const GOVERNANCE_MULTISIG: Pubkey = pubkey!("...");
#[account(constraint = signer.key() == &GOVERNANCE_MULTISIG)]
pub signer: Signer<'info>,
```

For full timelock semantics (delay between proposal and execution), the gate would need to be a CPI from a separate Timelock program (Squads, SPL Governance / Realms, or a custom Timelock). The simpler «admin Pubkey only» pattern is what most Solana governance v0s ship and is a reasonable starting point.

### Why this didn't blow up on mainnet

The mainnet check shows this program is not deployed and the mainnet lockbox-solana's upgrade authority is some other address (verified as a System-owned EOA, see §2.5 below). So the deployment team appears to have chosen the simpler «just point upgrade authority at an off-chain wallet» path, bypassing this governance-program layer entirely. That's a reasonable operational fallback — but it means **this code never matured to production**, and shipping it without first fixing the gate would re-introduce the vulnerabilities into mainnet.

---

## 2.5 Architectural-debt note — the missing cross-chain bridge layer

This section answers a question the rest of the audit can only point at: **what governance model was the developer actually trying to implement?** «Timelock governed» is the README's phrase, but Solana has no native Timelock pattern (Timelock is an OpenZeppelin Governance contract on Ethereum). So what did the developer have in mind?

### The Olas multi-chain governance pattern

Olas governs its L2 / sidechain / non-EVM deployments via a single canonical pattern:

```
L1 (Ethereum):
    OLAS holders vote
    →  GovernorOLAS (OZ Governance)
    →  Timelock (delay + queue)
    →  L1-side bridge sender contract  (e.g., WormholeRelayerTimelock.sol)
                                            ↓
                                       [Bridge guardians sign message]
                                            ↓
L2 (or non-EVM chain):
    L2-side bridge Messenger contract  (e.g., WormholeMessenger.sol on Polygon/Optimism/Celo;
                                              Solana-side analog for Solana)
        guards: msg.sender == bridge_relayer
                sourceChain  == L1_chain_id
                sourceAddress == L1_Timelock_address
    →  CPI / call into the target contract (here: fee_collector)
```

This pattern is implemented in `valory-xyz/autonolas-governance` for every EVM L2 / sidechain Olas governs. Reference contract: `contracts/bridges/WormholeMessenger.sol`. The relevant gate is at `receiveWormholeMessages(...)`, lines 80-95:

```solidity
function receiveWormholeMessages(
    bytes memory data, bytes[] memory,
    bytes32 sourceAddress, uint16 sourceChain, bytes32 deliveryHash
) external payable {
    if (msg.sender != wormholeRelayer) {                            // ← (1) only Wormhole Relayer can deliver
        revert TargetRelayerOnly(msg.sender, wormholeRelayer);
    }
    if (sourceChain != sourceGovernorChainId) {                     // ← (2) only from L1 (Ethereum)
        revert WrongSourceChainId(sourceChain, sourceGovernorChainId);
    }
    bytes32 governor = sourceGovernor;
    if (governor != sourceAddress) {                                // ← (3) only from L1 Timelock
        revert SourceGovernorOnly32(sourceAddress, governor);
    }
    // ... only now execute the relayed transaction
}
```

These **three checks** ARE the «Timelock governed» enforcement — the L2-side gate verifies that the relayed message originated from the L1 Olas Timelock and arrived via the canonical Wormhole bridge. The Timelock delay happens on L1; the L2 contract just verifies origin.

### Why this is almost certainly what the dev intended for fee_collector

Three pieces of evidence converge on this conclusion:

1. **The 2024-06 internal audit branch was named `bridging4`** (the historical branch holding `audits/internal/`; see §10 cross-refs). The literal name «bridging» indicates that the design context was cross-chain bridge integration.
2. **The architectural shape matches**: a dedicated «governor program» on the L2 side, holding fee accounts + upgrade authority, expecting to be invoked by a relayed message — exactly the shape of `WormholeMessenger.sol`'s peers on every other Olas-governed L2.
3. **The three `/// CHECK: Check later` markers in `change_upgrade_authority`'s account struct map cleanly to the three EVM-side checks:**
   - `program_to_update_authority: UncheckedAccount` — should be constrained to the BPF Upgradeable Loader's program-info account (analog of (1) — only bridge-relayer can deliver / only valid program metadata accepted)
   - `program_data_to_update_authority: UncheckedAccount` — should be constrained to match `program_to_update_authority`'s ProgramData PDA (analog of (2) — only the right L1 origin)
   - `destination: UncheckedAccount` — should be constrained to be the new authority specified in the relayed Timelock message (analog of (3) — origin == L1 Timelock; on Solana this manifests as «only the Wormhole gateway program's PDA can sign as the `signer`, AND the relayed payload determines the destination»)

The dev appears to have written the «governor program» skeleton intending to wire it up to a Wormhole gateway later. They never returned. The «CHECK: Check later» markers are placeholders for the bridge-integration checks that were postponed until the bridge wiring decision was made.

### What's missing on the Solana side (the bridge-integration gap)

**Solana-vs-EVM design note: the EVM pattern of «separate Messenger contract + separate Executor contract» does NOT translate directly to Solana.** EVM contracts are cheap to deploy (just gas) and composability via inter-contract calls is the dominant idiom, so EVM Olas ships `WormholeMessenger.sol` as a dedicated contract. Solana programs are different: deploy cost is non-trivial (rent + buffer accounts), each program adds operational overhead (its own upgrade ceremony, its own program ID, its own state), and the Solana ecosystem strongly prefers **fewer, larger programs over many small composable ones**. Mechanically porting the EVM «two-contract» structure into «two Solana programs» would be an EVM-anti-pattern on Solana.

The Wormhole protocol on Solana is already aware of this and ships a **single shared Wormhole Core program** at the well-known address `worm2ZoG2kUd4vFXhvjh93UUH596ayRfgQ2MgjNMTth`. Every Solana program that wants to consume bridged messages reads from this shared core — no per-protocol Messenger deployment needed. The idiomatic Solana flow:

1. **Off-chain relayer** posts a Wormhole VAA to the Wormhole Core program (one-time, via `post_vaa`); the core program parses + stores the posted VAA in a Core-owned account, attesting that guardians signed it.
2. **fee_collector instruction** is then called with the posted-VAA account passed in as an input account.
3. **fee_collector reads + verifies the VAA inline** (no separate Messenger program; no CPI into a bespoke gateway):
   - Verify `vaa_account.owner == WORMHOLE_CORE_PROGRAM_ID` (the posted VAA must come from the legitimate Wormhole Core).
   - Deserialize the posted-VAA payload.
   - Check `vaa.emitter_chain == collector.source_chain_id` (Ethereum).
   - Check `vaa.emitter_address == collector.source_governor` (Olas Timelock on Ethereum).
   - Check `vaa.consistency_level` and replay defense (mark VAA hash as consumed).
   - Decode the action from the VAA payload and execute it directly.

The whole flow lives in **one program** (`fee_collector`). This is the Solana-idiomatic equivalent of the EVM «Messenger + Executor» pair — collapsed into a single program that knows how to consume bridged messages and act on them.

So the state additions to `FeeCollector` are minimal:

```rust
pub struct FeeCollector {
    pub bump: [u8; 1],

    // [MISSING] L1 chain ID (Ethereum = Wormhole chain ID 2). The posted VAA
    //           must come from this chain.
    pub source_chain_id: u16,

    // [MISSING] L1 source-governor address (Olas Timelock on Ethereum).
    //           The posted VAA's emitter MUST equal this.
    pub source_governor: [u8; 32],

    // [MISSING] Replay protection — set of consumed VAA hashes (bounded;
    //           or a circular buffer like processed_tx in other Solana
    //           protocols).
    pub consumed_vaas: /* ring buffer of [u8; 32] */,

    // (existing) dead state fields per F-4 — should be repurposed for
    // counting bridge-executed transfers
    pub total_sol_transferred: u64,
    pub total_olas_transferred: u64,
}
```

And the per-instruction account structure adds the posted-VAA account (Wormhole Core program ID is hardcoded as a `const` like SOL / OLAS — F-11):

```rust
const WORMHOLE_CORE_PROGRAM_ID: Pubkey = pubkey!("worm2ZoG2kUd4vFXhvjh93UUH596ayRfgQ2MgjNMTth");

pub struct TransferFeeCollector<'info> {
    // Anyone can be the signer (it's just a relayer paying for the tx).
    // The gate is on the VAA, not on the signer.
    pub signer: Signer<'info>,

    // The posted VAA account must be owned by the Wormhole Core program
    // (so we know guardians signed it). fee_collector then deserializes
    // the payload, verifies emitter_chain + emitter_address match
    // collector.source_chain_id / source_governor, runs replay defense,
    // and only then executes the requested action.
    /// CHECK: owner-checked at instruction entry against
    ///        WORMHOLE_CORE_PROGRAM_ID; payload + emitter verified
    ///        against collector state.
    pub posted_vaa: UncheckedAccount<'info>,
    /* ... rest of accounts: collector_account, destination, etc. ... */
}
```

This **inverts the «no signer check» problem** from §F-1 / §F-2 / §F-3: the `signer` is genuinely just the relayer (anyone), but the GATE is now «posted VAA exists, came from Olas L1 Timelock, hasn't been replayed». The three `/// CHECK: Check later` markers ARE the bridge-integration checks the dev planned to add — but the idiomatic Solana form is «inline VAA verification in this same program», not «separate gateway program with PDA-signed CPI».

**The current `fee_collector` has NONE of this.** No `source_chain_id`, no `source_governor`, no posted-VAA verification, no Wormhole Core program reference, no replay defense, no payload decoding. The bridge-integration scope is real work, but it's all WITHIN this one program — no second program needed.

### Operational reality vs intended design

| Aspect | Intended (Olas multi-chain pattern) | Current mainnet reality |
|---|---|---|
| L1 Timelock vote | Ethereum OLAS Governor + Timelock | OLAS Governor + Timelock exist on Ethereum ✓ |
| Bridge transport | Wormhole VAA L1 → Solana | **Not implemented** |
| L2 Executor | This `fee_collector` with bridge-origin checks | **fee_collector not deployed; bridge checks absent** |
| Solana `lockbox-solana` upgrade authority | This `fee_collector` PDA (controlled by bridge-relayed Timelock messages) | EOA `7mQQw6qCj66EtfarNKUfWGUD1pX7SSjHbVMpdJc1roK2` — single off-chain wallet owned by the team, holds upgrade authority directly |

So the architectural intent was clearly (a) — Olas L1 Timelock → Wormhole bridge → Solana fee_collector → controls lockbox-solana. The current deployment is essentially **«hot-wallet bypass»** of the entire L2-Executor layer. Mainnet has been running this way for 21 months because the bridge-integration work was never completed.

### Why this matters beyond «fix the access control»

Even if the simple per-instruction admin-check fix from §1 (add `governance_authority: Pubkey` + `constraint = signer.key() == &collector.governance_authority`) is applied, it would NOT match the intended Olas governance model. A simple admin-check makes `fee_collector` a multisig-like program, not a bridge-relayed-L1-Timelock executor. The architectural choice is:

- **Path A (full bridge integration)** — implement the Wormhole gateway → fee_collector flow with full origin verification. This matches Olas multi-chain governance discipline. Significant work; requires Wormhole integration on Solana side.
- **Path B (simple Solana-native admin gate)** — store a `governance_authority` Pubkey (a Squads multisig or any chosen admin), gate on `signer == governance_authority`. NOT the original intent; Solana-governance-only; would mean Olas Solana lives on a different governance model than Olas EVM chains.
- **Path C (retire)** — formal deprecation; mainnet `lockbox-solana` keeps using the EOA-controlled `7mQQw6q...` upgrade authority indefinitely. Honest about what's actually happening, but doesn't fix the EOA centralization risk on lockbox-solana itself.

**Path A is the architecturally consistent answer** — it preserves the «Olas governance is unified across all chains via Ethereum Timelock» property that the Olas multi-chain pattern is built around. It's also what the dev appears to have started building before stalling. The implementation gap is the missing Wormhole VAA verification + state-field plumbing described above.

### Severity recalibration in light of architectural debt

The F-1 / F-2 / F-3 Critical findings remain Critical at the code level, but the root cause is more accurate to describe as «bridge-integration was never completed; placeholder access-control was left as-is». The fix is not «add an admin check» but «complete the bridge-integration flow per Olas multi-chain pattern, OR explicitly choose a simpler governance model and document the departure from the Olas pattern». The audit's recommendation is to make this architectural decision before any code-level fix work begins.

---

## 3. Inventory

| File | LoC | Role |
|---|---|---|
| `programs/fee_collector/src/lib.rs` | 209 | All instruction logic (`initialize`, `transfer`, `transfer_token_account`, `change_upgrade_authority`) + 4 account-validation structs + error enum (1 unused variant) + event struct (never emitted) |
| `programs/fee_collector/src/state.rs` | 33 | `FeeCollector` PDA struct (bump, total_sol_transferred, total_olas_transferred — last two never updated by any instruction) |
| **Total** | **242** | |

Program ID: `DWDGo2UkBUFZ3VitBfWRBMvRnHr7E2DSh57NK27xMYaB` (declared in `lib.rs:15`, matches `Anchor.toml:4`). Not currently deployed on mainnet (verified 2026-06-03).

---

## 4. Findings

### F-1 — `transfer` instruction has NO access control (Critical-class design defect; dormant)

**Severity:** Critical (design); dormant per §1 caveat
**Anchor:** `programs/fee_collector/src/lib.rs:47-67` (instruction body) + `lib.rs:135-149` (`TransferFeeCollector` accounts struct)

```rust
pub fn transfer(
    ctx: Context<TransferFeeCollector>,
    amount: u64
) -> Result<()> {
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.collector_account.to_account_info(),
                to: ctx.accounts.destination.to_account_info(),
                authority: ctx.accounts.collector.to_account_info(),
            },
            &[&ctx.accounts.collector.seeds()],   // ← PDA signs the transfer
        ),
        amount,
    )?;
    Ok(())
}

pub struct TransferFeeCollector<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,                    // ← ANY signer; no admin check
    #[account(mut)]
    pub collector: Box<Account<'info, FeeCollector>>,
    #[account(mut)]
    pub collector_account: Box<Account<'info, TokenAccount>>,  // ← ANY token account
    #[account(mut)]
    pub destination: Box<Account<'info, TokenAccount>>,         // ← ANY destination
    #[account(address = token::ID)]
    pub token_program: Program<'info, Token>
}
```

**Exploit construction**:

1. Identify any SPL TokenAccount X whose `authority` field is the `fee_collector` PDA (e.g., a fee-receiving account that was migrated to this PDA via `transfer_token_account`).
2. Construct your own SPL TokenAccount Y to receive the funds.
3. Call `program.methods.transfer(amount).accounts({ signer: <your wallet>, collector: <PDA>, collector_account: X, destination: Y, token_program: TOKEN_PROGRAM_ID }).rpc()`.
4. The PDA-signed CPI executes; X's balance flows to Y up to `amount`.

**No signer check, no destination check, no mint check.** Anyone with a Solana wallet can drain any token account owned by this PDA.

**Why dormant in current production**: the program is not deployed; if it were deployed, the deployer would still have to migrate token-account ownership to this PDA via `transfer_token_account` (which is also broken — see F-2) for there to be anything to drain.

**Fix recommendation**:

```rust
pub struct TransferFeeCollector<'info> {
    #[account(constraint = signer.key() == &collector.governance_authority)]
    //                     ^^^^^^^^^^^^^^^^^^^ require signer be the
    //                                          stored governance authority
    pub signer: Signer<'info>,
    /* ... */
}
```

Plus add `governance_authority: Pubkey` to `FeeCollector` state set at init time. Plus consider constraining `collector_account.mint` to the SOL / OLAS hardcoded constants (which currently exist but are unused — F-11) for defense in depth.

---

### F-2 — `transfer_token_account` instruction has NO access control + destination is UncheckedAccount (Critical-class design defect; dormant)

**Severity:** Critical (design); dormant per §1 caveat
**Anchor:** `lib.rs:70-93` (instruction body) + `lib.rs:152-167` (`TransferTokenAccountFeeCollector` accounts struct)

```rust
pub fn transfer_token_account(
    ctx: Context<TransferTokenAccountFeeCollector>
) -> Result<()> {
    invoke_signed(
        &set_authority(
            ctx.accounts.token_program.key,
            ctx.accounts.collector_account.to_account_info().key,
            Some(ctx.accounts.destination.to_account_info().key),
            AuthorityType::AccountOwner,             // ← changes OWNER of the token account
            ctx.accounts.collector.to_account_info().key,
            &[ctx.accounts.collector.to_account_info().key],
        )?,
        /* ... */
        &[&ctx.accounts.collector.seeds()],          // ← PDA signs
    )?;
    Ok(())
}

pub struct TransferTokenAccountFeeCollector<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,                       // ← ANY signer

    #[account(mut)]
    pub collector: Box<Account<'info, FeeCollector>>,
    #[account(mut)]
    pub collector_account: Box<Account<'info, TokenAccount>>,  // ← ANY token account

    /// CHECK: Check later                                       // ← marker that validation
    #[account(mut)]                                              //    was DEFERRED — never landed
    pub destination: UncheckedAccount<'info>,                    // ← ANY pubkey, not even
                                                                 //    a TokenAccount
    #[account(address = token::ID)]
    pub token_program: Program<'info, Token>
}
```

**Exploit construction**: any signer can pass any SPL TokenAccount owned by the PDA as `collector_account` and any pubkey as `destination`, then call this instruction. The CPI to SPL Token `set_authority` changes the account's owner to the attacker's address. After this, the attacker — not the PDA — owns the token account and can spend its balance directly.

This is the **mechanism by which fee accounts get migrated TO the PDA** in the design intent (the lockbox-solana audit observed that fee_collector accounts are initially owned by the initializer; the intent is that they then get `set_authority`-ed to this PDA's control). The same instruction, with no signer check, allows them to be migrated AWAY to anyone.

**Plus**: the «CHECK: Check later» marker indicates the dev was aware that destination needs validation and deferred it. Even basic validation like «destination must be a TokenAccount» or «destination must match the same mint» is missing.

**Fix recommendation**: same as F-1 — add `governance_authority` check on `signer`. Plus consider replacing `UncheckedAccount<'info>` with a typed `Account<'info, TokenAccount>` and adding a mint-equality constraint:

```rust
#[account(constraint = destination.mint == collector_account.mint)]
pub destination: Box<Account<'info, TokenAccount>>,
```

---

### F-3 — `change_upgrade_authority` instruction has NO access control + three UncheckedAccount with «CHECK: Check later» (Critical-class design defect; dormant)

**Severity:** Critical (design); dormant per §1 caveat
**Anchor:** `lib.rs:96-117` (instruction body) + `lib.rs:170-188` (`ChangeUpgradeAuthorityFeeCollector` accounts struct)

```rust
pub fn change_upgrade_authority(
    ctx: Context<ChangeUpgradeAuthorityFeeCollector>
) -> Result<()> {
    invoke_signed(
        &set_upgrade_authority(
            ctx.accounts.program_to_update_authority.to_account_info().key,
            ctx.accounts.collector.to_account_info().key,
            Some(ctx.accounts.destination.to_account_info().key)
        ),
        /* ... */
        &[&ctx.accounts.collector.seeds()]
    )?;
    Ok(())
}

pub struct ChangeUpgradeAuthorityFeeCollector<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,                                 // ← ANY signer

    /// CHECK: Check later                                       // ← deferred validation #1
    #[account(mut)]
    pub program_to_update_authority: UncheckedAccount<'info>,  // ← ANY program

    /// CHECK: Check later                                       // ← deferred validation #2
    #[account(mut)]
    pub program_data_to_update_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub collector: Box<Account<'info, FeeCollector>>,

    /// CHECK: Check later                                       // ← deferred validation #3
    #[account(mut)]
    pub destination: UncheckedAccount<'info>,                  // ← ANY destination
}
```

**Three** «CHECK: Check later» comments. None of them honored.

**Exploit construction**:
1. Identify any upgradeable Solana program whose upgrade authority is THIS PDA's key.
2. Pass that program as `program_to_update_authority` + its corresponding ProgramData PDA as `program_data_to_update_authority` + your own pubkey as `destination`.
3. Call `change_upgrade_authority`. The PDA-signed CPI to BPF Upgradeable Loader `set_upgrade_authority` transfers upgrade authority from the PDA to your pubkey.
4. You can now deploy any malicious bytecode as the next program-upgrade and steal everything that program holds.

For an Olas mainnet scenario where `lockbox-solana`'s upgrade authority WAS routed through this PDA: attacker takes upgrade authority → deploys malicious lockbox bytecode → drains all locked OLAS+SOL.

**Why dormant on current mainnet**: per §1 caveat, the mainnet `lockbox-solana` upgrade authority is `7mQQw6qCj66EtfarNKUfWGUD1pX7SSjHbVMpdJc1roK2`, NOT this PDA. The PDA isn't even deployed. So the attack has no live target. **But the code is the exact mechanism that would be live-exploitable the moment any program assigns its upgrade authority to this PDA in this state.**

**Fix recommendation**: same as F-1 and F-2 — `governance_authority` check on signer. **Plus** type the `program_to_update_authority` / `program_data_to_update_authority` properly (Anchor has helpers for this; alternatively, store an allow-list of programs this fee_collector can act on and verify at instruction-time). **Plus** at minimum verify that `program_to_update_authority.owner == BPFLoaderUpgradeable::id()` (currently any account passes).

---

### F-4 — `total_sol_transferred` / `total_olas_transferred` state fields never updated (Info)

**Severity:** Info (dead state)
**Anchor:** `state.rs:6-9` (fields declared) + `lib.rs` (no instruction writes to these)

```rust
pub struct FeeCollector {
    pub bump: [u8; 1],
    pub total_sol_transferred: u64,        // ← never updated by any instruction
    pub total_olas_transferred: u64        // ← never updated
}
```

`grep -rn 'total_sol_transferred\|total_olas_transferred' programs/` returns hits ONLY in `state.rs:7-9` (declaration) and `state.rs:24-25` (initialize to 0). No instruction writes. The fields permanently read 0.

Either delete them, or add update logic to `transfer` (e.g., distinguish SOL-mint transfers and OLAS-mint transfers via a constraint on `collector_account.mint`, then increment the corresponding counter). The TransferEvent (F-5) appears to have been designed to emit these values; coordinated update + emit would make the state and event consistent.

---

### F-5 — `TransferEvent` declared but never emitted (Info)

**Severity:** Info (dead code)
**Anchor:** `lib.rs:196-204`

```rust
#[event]
pub struct TransferEvent {
    #[index] pub signer: Pubkey,
    pub sol_transferred: u64,
    pub olas_transferred: u64
}
```

`emit!(TransferEvent { ... })` does not appear anywhere in the codebase. Dead code, paired with F-4.

---

### F-6 — `ErrorCode::WrongTokenMint` never used (Info)

**Severity:** Info (dead code)
**Anchor:** `lib.rs:190-194`

```rust
pub enum ErrorCode {
    #[msg("Wrong token mint")]
    WrongTokenMint,
}
```

The single variant is referenced from nowhere. The intent was presumably to enforce `collector_account.mint == SOL || OLAS` — but the constants are declared (lib.rs:21-23) and the error is declared, and the check is missing. F-11 covers the unused constants angle.

---

### F-7 — Unused imports (Info)

**Severity:** Info (hygiene)
**Anchor:** `lib.rs:3-12`

```rust
use anchor_spl::token::{self, Mint, Token, TokenAccount, Approve, Transfer};
//                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ unused
use solana_program::{
    pubkey::Pubkey,
    program::invoke_signed,
    bpf_loader_upgradeable::set_upgrade_authority,
    bpf_loader_upgradeable::upgrade,                       // ← unused
    loader_upgradeable_instruction::UpgradeableLoaderInstruction  // ← unused
};
use spl_token::instruction::{transfer, set_authority, AuthorityType};
//                            ^^^^^^^^ unused (uses anchor_spl::token::Transfer instead)
```

`Mint`, `Approve` and `spl_token::instruction::transfer` are imported but not referenced. `bpf_loader_upgradeable::upgrade` and `UpgradeableLoaderInstruction` are imported but not referenced (the upgrade-related logic uses only `set_upgrade_authority`). `cargo build` will emit `unused_imports` warnings.

---

### F-8 — Toolchain stale (Anchor 0.26.0 / Solana 1.14.29 / Rust 1.62) + cargo audit dev-time vulnerabilities (Info)

**Severity:** Info (transitive dev-time deps; NOT in program binary)
**Source:** `cargo audit` against `Cargo.lock`

Same situation as the sister Solana repos: toolchain pinned at mid-2022 versions. `cargo audit` flags:
- `curve25519-dalek 3.2.1` (RUSTSEC-2024-0344, timing variability)
- `ed25519-dalek 1.0.1` (RUSTSEC-2022-0093, double-pubkey signing oracle)
- `atty`, `bincode`, `libsecp256k1`, `borsh`, `keccak`, `rand`, `bytemuck` — various unmaintained / advisory warnings

All transitive dev-time deps; on-chain program binary does not include them (Solana runtime handles signature verification via syscalls). Runtime exposure = 0. The pin is reasonable because of Anchor / Solana cross-version compatibility constraints; clears naturally with toolchain refresh whenever Orca / Solana / Anchor mature.

---

### F-9 — On-chain bytecode-vs-source + governance-authority verification (Info; recommended follow-up)

**Severity:** Info (out-of-scope for source audit; flagged for completeness)
**Verified 2026-06-03**:
- `DWDGo2UkBUFZ3VitBfWRBMvRnHr7E2DSh57NK27xMYaB` (this program ID) — **NOT DEPLOYED on Solana mainnet** as of audit date.
- `1BoXeb8hobfLCHNsyCoG1jpEv41ez4w4eDrJ48N1jY3` (mainnet `lockbox-solana`) — ProgramData PDA `4db7h3HiTf8DHe5WYLbkEFiG7J7R5xJKqvHvpWuz8KXc` shows upgrade authority `7mQQw6qCj66EtfarNKUfWGUD1pX7SSjHbVMpdJc1roK2` (NOT this fee_collector PDA).

These two facts together establish the «dormant» qualifier on F-1, F-2, F-3 — there is currently no production deployment that this code's defects can compromise.

**Recommended follow-up:** if this program is ever planned for production deployment (e.g., as the Olas v2 Solana governance layer), the audit MUST be re-run after the access-control fixes land. The current audit is a code-quality verdict; deployment of this code as-is would be a Critical-class production risk.

Also recommended: investigate what `7mQQw6qCj66EtfarNKUfWGUD1pX7SSjHbVMpdJc1roK2` is (Squads multisig PDA? DAO multisig? EOA?). If it's an EOA, that's its own centralization concern (not in scope of this audit but worth surfacing at the project level).

---

### F-10 — `initialize` has no authentication (Info; design-time)

**Severity:** Info
**Anchor:** `lib.rs:30-44` + `lib.rs:121-133`

```rust
pub struct InitializeFeeCollector<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,                          // ← ANY signer
    #[account(init, seeds = [b"fee_collector".as_ref()], bump,
              payer = signer, space = FeeCollector::LEN)]
    pub collector: Box<Account<'info, FeeCollector>>,
    /* ... */
}

pub fn initialize(ctx: Context<InitializeFeeCollector>) -> Result<()> {
    let collector = &mut ctx.accounts.collector;
    let bump = *ctx.bumps.get("collector").unwrap();
    collector.initialize(bump)?;                        // ← does NOT store admin
    Ok(())
}
```

`init` ensures the PDA can only be created once (idempotency-fail on re-initialization), but the FIRST caller wins. There's no admin / governance authority stored at init. So even if the gate were added to the privileged ops (F-1/F-2/F-3 fixes), the gate's reference value (`governance_authority`) wouldn't be set anywhere.

**Recommendation as part of F-1/F-2/F-3 fix:** add a `governance_authority: Pubkey` to `FeeCollector` state, set it at init time (either from a hardcoded constant via `address = ADMIN_PUBKEY` constraint on signer, OR by storing the signer at init time, OR by accepting `governance_authority: Pubkey` as an instruction parameter). The choice affects the trust model:

- **Hardcoded admin via const**: simplest; admin can never change without redeploying.
- **Store signer at init**: deployer becomes admin; can add `change_admin` instruction gated by current admin.
- **Parameter at init**: deployer can hand off admin role at init time (e.g., to a multisig PDA they computed off-chain).

Any of these is much better than current state of «no admin field at all».

---

### F-11 — `SOL` and `OLAS` constants declared but never used (Info)

**Severity:** Info (dead code; misleading)
**Anchor:** `lib.rs:21-23`

```rust
const SOL: Pubkey = pubkey!("So11111111111111111111111111111111111111112");
const OLAS: Pubkey = pubkey!("Ez3nzG9ofodYCvEmw73XhQ87LWNYVRM2s7diB5tBZPyM");
```

Neither constant is referenced anywhere else in the program. Their existence implies the program enforces SOL/OLAS-only operation, but the code doesn't — `transfer`, `transfer_token_account` accept any mint.

**Recommendation**: enforce `collector_account.mint == SOL || collector_account.mint == OLAS` in `TransferFeeCollector` / `TransferTokenAccountFeeCollector` constraints (companion to F-1/F-2 fixes), so the constants gain teeth. This also gives `ErrorCode::WrongTokenMint` (F-6) a use.

---

## 5. Anchor security pattern checklist

| # | Pattern | Status | Notes |
|---|---|---|---|
| A-01 | PDA seed verification (`seeds = true`) | ✓ PASS | Anchor.toml enables; PDA derivation correct |
| A-02 | Signer enforcement | ⚠ INCOMPLETE | `Signer<'info>` present but no address-constraint on privileged ops — F-1, F-2, F-3 |
| A-03 | `init` PDA space matches struct serialized size | ✓ PASS | `FeeCollector::LEN = 8 + 1 + 8*2 = 25` matches discriminator + bump + 2×u64 |
| A-04 | `init_if_needed` careful usage | ✓ N/A | Plain `init`; no re-init surface |
| A-05 | `realloc` safety | ✓ N/A | No realloc |
| A-06 | CPI safety (callee program ID + signer seeds) | ⚠ PARTIAL | SPL Token CPI uses `address = token::ID` constraint ✓; BPF Upgradeable Loader CPI has NO program-ID check on `program_to_update_authority` — F-3 |
| A-07 | Owner / discriminator checks | ⚠ PARTIAL | Anchor's `Account<'info, T>` checks discriminator + owner ✓; but several UncheckedAccount with «CHECK: Check later» bypass this — F-2, F-3 |
| A-08 | Account mutability declarations | ✓ PASS | All `mut` correctly applied |
| A-09 | Overflow checks (`overflow-checks = true`) | ✓ PASS | Set in root + program Cargo.toml |
| A-10 | Reentrancy | ✓ N/A | Solana runtime forbids cross-program reentry |
| A-11 | Closing account / lamports drain | ✓ N/A | No close instruction |
| A-12 | Token authority transfer correctness | ❌ FAIL | `transfer_token_account` calls `set_authority(AccountOwner)` with NO signer gate — F-2 |
| A-13 | Sysvars usage | ✓ PASS | Rent / SystemProgram declared per Anchor convention |
| A-14 | `declare_id!` vs deployed program | ⚠ N/A on-chain | `declare_id!` matches Anchor.toml; mainnet deployment absent per F-9 |
| A-15 | Bump persistence (avoid re-`find_program_address`) | ✓ PASS | `FeeCollector.bump` stored + used by `seeds()` |
| A-16 | UncheckedAccount with «CHECK:» comment justification | ❌ FAIL | Three UncheckedAccount marked `/// CHECK: Check later` — validation never landed — F-2, F-3 |
| A-17 | Privileged-instruction admin gating | ❌ FAIL | Three privileged instructions; zero admin checks — F-1, F-2, F-3 |
| A-18 | Constants used vs declared | ⚠ MINOR | SOL / OLAS declared but never referenced — F-11 |

**3 FAIL items (A-12, A-16, A-17) drive the 3 Critical-class findings.**

---

## 6. Cross-domain pattern sweep

| Pattern | Applicability | Status |
|---|---|---|
| Authority bypass / privilege escalation | ✓ in-scope (admin instructions present) | **FAIL** — F-1, F-2, F-3 |
| Re-initialization | ✓ | PASS — `init` used; one-shot |
| Front-running on creation | ✓ | PASS at instruction level — but F-10 design-time note (first caller wins on init; if admin were stored from signer, that's «whoever runs initialize first becomes admin»). |
| Arithmetic overflow | N/A | No arithmetic in implemented code |
| CPI program-ID validation | ✓ | PARTIAL — Token program ID checked (F-3 still FAILS for BPF Upgradeable Loader target) |
| UncheckedAccount over-trust | ✓ | **FAIL** — three CHECK-deferred-never-landed |
| Dead-state / dead-event | ✓ | F-4 + F-5 (dead state fields + unused event) |
| Constants declared without use | ✓ | F-11 (SOL / OLAS) |

---

## 7. Adversarial sweep — what's NOT in the dev's claims

Three observations:

1. **README claims «Timelock governed» — code has zero timelock enforcement.** This is the central documentation-vs-code mismatch. Either the README needs to be reworded to «Timelock governed (intent; v0 ships without gate; do not deploy until added)», or the gate needs to be added.
2. **«Check later» markers should not survive to production audit.** Three `/// CHECK: Check later` markers across the privileged-instruction account structs are explicit author-time TODOs. A code-review discipline that catches «CHECK: Check later» before merge to main would have surfaced this earlier.
3. **The mainnet deployment-team appears to have chosen «use multisig as direct upgrade authority» instead of «route through governance program».** The fact that mainnet `lockbox-solana` upgrade authority is `7mQQw6qCj66EtfarNKUfWGUD1pX7SSjHbVMpdJc1roK2` (NOT this PDA) suggests an operational decision to bypass this program entirely. This is fine as a fallback, but it means this governance program is effectively unmaintained code that hasn't been used in production. The fact that it sat for 21 months between the last code commit (2024-04-05) and this audit (2026-06-03) reinforces that nobody is actively maintaining it. Either retire it formally (mark deprecated in README) or fix it and bring it into production.

---

## 8. Compliance report

| Item | Status |
|---|---|
| All `.rs` files read (242 LoC, 2 files) | ✓ |
| Exhaustive Anchor + Solana + cross-domain checklist | ✓ (§5 + §6) |
| Adversarial sweep | ✓ (§7) |
| README claims verified against actual code (VCE) | ✓ — «Timelock governed» claim verified to be unbacked by enforcement |
| `cargo audit` run | ✓ (F-8) |
| Grep sweep for hidden code | ✓ — confirmed minimal surface |
| On-chain mainnet state checked | ✓ — F-9 verifies program not deployed; lockbox-solana upgrade authority routed elsewhere |
| Verdict tied to evidence; severity calibrated against operational reality | ✓ — 3 Criticals labeled «design defect; dormant in production» with explicit caveat |

---

## 9. Forward-look

**The fork in the road is described in §2.5 «Architectural-debt note» — Path A (full bridge integration matching Olas multi-chain pattern), Path B (simple Solana-native admin gate), or Path C (formal retirement).** The recommendation is Path A as the architecturally consistent answer, but the choice is the project's to make.

**If Path A — complete the bridge integration (Olas-pattern consistent; recommended):**
1. Add `source_chain_id` (`u16`), `source_governor` (`[u8; 32]`), and a VAA-replay-protection structure (e.g. `consumed_vaas` ring buffer) to `FeeCollector` state at init. Hardcode `WORMHOLE_CORE_PROGRAM_ID` as a module-level `const` (alongside SOL / OLAS — F-11).
2. **Single program, not two**: keep all bridge-message handling INSIDE `fee_collector`. Each privileged instruction accepts a `posted_vaa: UncheckedAccount` and verifies it inline (owner == Wormhole Core; emitter_chain == `collector.source_chain_id`; emitter_address == `collector.source_governor`; VAA hash not yet consumed; payload decodes to the requested action). The Solana idiomatic alternative to «separate Messenger contract» is inline VAA consumption — see §2.5 for the rationale (Solana prefers fewer/larger programs; Wormhole Core is a shared deployment already on-chain).
3. F-2, F-3 UncheckedAccount fields tighten: the bridge-relayed payload determines `destination` / `program_to_update_authority` etc.; the program reads them from the verified VAA payload rather than trusting caller-supplied account inputs (the three `/// CHECK: Check later` markers correspond to fields that should be VAA-derived).
4. F-4 / F-5 state-update + event-emit coordinated implementation (the `TransferEvent` becomes meaningful once it fires from bridge-relayed Timelock-authorized transfers).
5. F-7 unused imports cleanup; F-11 SOL/OLAS mint enforcement gives F-6 a use.
6. Tests: extend existing happy-path tests with adversarial tests verifying — (a) direct call without a posted VAA rejects; (b) VAA from wrong emitter_chain rejects; (c) VAA from wrong emitter_address rejects; (d) VAA replay rejects; (e) malformed payload rejects.
7. Coordinated mainnet rollout: deploy `fee_collector`, then via legitimate Olas L1 Timelock governance proposal, transfer `lockbox-solana` upgrade authority FROM the `7mQQw6q...` EOA TO the `fee_collector` PDA. Chicken-and-egg: this first transfer must be initiated by the off-chain key holder of `7mQQw6q...` (it can't be relayed via the bridge yet since fee_collector isn't yet wired in).
8. Re-audit MANDATORY before any of the above is deployed.

**If Path B — simple Solana-native admin gate (acceptable but not Olas-pattern consistent):**
1. Add `governance_authority: Pubkey` to `FeeCollector` state at init; gate every privileged instruction by `constraint = signer.key() == &collector.governance_authority`. Plus type the UncheckedAccount fields where reasonable.
2. Document explicitly in README that Solana governance does NOT mirror Olas EVM governance via Ethereum Timelock; it lives on a Squads multisig (or similar) by design. Reword «Timelock governed» to match reality.
3. F-4 / F-5 / F-7 / F-11 as above.
4. Re-audit MANDATORY.

**If Path C — formal retirement:**
1. Mark deprecated in README.
2. Document that mainnet `lockbox-solana` upgrade authority remains the off-chain `7mQQw6q...` EOA, and the governance model is «Olas team multisig (off-chain) + dispatch-by-discipline timelock». This is a centralization disclosure but at least honest.
3. Optionally archive the repo on GitHub.
4. The F-1 / F-2 / F-3 Critical findings become moot at the project level (no production deployment path for the broken code). Separate centralization concern on `7mQQw6q...` itself is a project-level architectural issue, not a code-level finding against this audit's scope.

**Either way:** the current state — README claims governance discipline matching the Olas multi-chain pattern, code has neither bridge integration nor admin check, repo sits dormant 21 months, mainnet `lockbox-solana` runs on an off-chain EOA — is the worst-of-three-paths. Pick a direction.

---

## 10. Cross-references

- Source under audit: `programs/fee_collector/` at HEAD `735802d` (2026-05-25)
- Stated mainnet program ID: `DWDGo2UkBUFZ3VitBfWRBMvRnHr7E2DSh57NK27xMYaB` — **not deployed** as of 2026-06-03
- Mainnet `lockbox-solana` upgrade authority (verified 2026-06-03): `7mQQw6qCj66EtfarNKUfWGUD1pX7SSjHbVMpdJc1roK2` (NOT this PDA)
- ProgramData PDA of mainnet `lockbox-solana`: `4db7h3HiTf8DHe5WYLbkEFiG7J7R5xJKqvHvpWuz8KXc`
- Prior internal audit branch: `bridging4` (2024-06; the branch contains the historical `audits/internal/` work but was never merged to main) — the literal branch name «bridging» is consistent with the Wormhole bridge integration intent analyzed in §2.5
- Sister Solana repos (separate audits): `lockbox-solana` (audited as `audits/internal2/` 2026-06-02), `registries-solana` (audited as `audits/internal1/` 2026-06-02)
- Architectural reference: README at repo root claims «Timelock governed OLAS and SOL Fee Collector on Solana»
- **Olas multi-chain governance pattern reference**: `valory-xyz/autonolas-governance/contracts/bridges/WormholeMessenger.sol` — the canonical L2-Executor implementation used on every EVM L2 / sidechain Olas governs. `receiveWormholeMessages(...)` lines 80-95 contain the three checks (msg.sender == bridge_relayer + sourceChain == L1 + sourceAddress == L1 Timelock) that form the «Timelock governed» enforcement on the L2 side. Equivalent Solana implementation is what this `fee_collector` was almost certainly intended to be — see §2.5.

---

*audit-claude, 77ph, 2026-06-03*
