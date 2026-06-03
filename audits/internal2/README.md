# Internal audit 2 — `valory-xyz/lockbox-governor-solana`

**Date:** 2026-06-03
**Auditor:** audit-claude, 77ph
**Audit scope (two work-streams in the same repo):**
- `main` branch HEAD `735802d` — production-default `fee_collector` program, 2 `.rs` / 242 LoC
- `bridging4` branch HEAD `dd901fb` — `lockbox_governor` program from open PR #1 (2024-06-12), 8 `.rs` / 1345 LoC
**Toolchain (per repo `setup-env.sh`):** rustc 1.62.0, Solana 1.14.29, Anchor 0.26.0
**Mainnet program ID:** `DWDGo2UkBUFZ3VitBfWRBMvRnHr7E2DSh57NK27xMYaB` — **NOT DEPLOYED on Solana mainnet** (verified via RPC `getAccountInfo`, 2026-06-03; account returns null)
**Mainnet `lockbox-solana` upgrade authority:** `7mQQw6qCj66EtfarNKUfWGUD1pX7SSjHbVMpdJc1roK2` — System-owned EOA (single off-chain wallet); **NOT this `lockbox-governor` program's PDA**. Verified via ProgramData PDA `4db7h3HiTf8DHe5WYLbkEFiG7J7R5xJKqvHvpWuz8KXc` parsed `info.authority` field, 2026-06-03.

---

## 1. Verdicts

| Work-stream | Verdict | Summary |
|---|---|---|
| **`main` — `fee_collector`** | **FAIL** | 3 Critical-class design defects (no access control on 3 admin instructions); 8 Info. Dormant in production because program isn't deployed and isn't referenced by mainnet lockbox-solana. |
| **`bridging4` — `lockbox_governor` (PR #1)** | **NEEDS REWORK** | Bridge-origin gates correctly implemented; 1 High-class explicit acknowledged TODO; 1 Medium structural (Wormhole Core program ID trust is implicit); 1 Low (Received PDA growth); several Info. Substantively closer to mergeable than `main` is fixable. |
| **Project-level decision required** | **YES** | The project has two divergent implementations of the same concept that have coexisted for ≈2 years. PR #1 has been open since 2024-06-12 without merge. The team needs to choose: merge PR #1 after fixes, abandon PR #1 and rebuild on `main`, or formally retire the program entirely (mainnet already uses an off-chain EOA for upgrade authority). |

---

## 2. Project intent

This program is intended to be the **L2-Executor for Olas governance on Solana**: a separate program that holds (a) the SPL token accounts where collected fees accumulate from `lockbox-solana` withdrawals, and (b) upgrade authority over `lockbox-solana` (and potentially over itself and over `registries-solana` if those are also routed through it). Governance decisions originate from the Olas DAO on Ethereum, are queued through the OZ Timelock, and are relayed to Solana via Wormhole. The Solana program's job is to verify each relayed message (VAA) and execute the requested action.

This shape — a separate program receiving bridge-relayed L1 governance messages — is the canonical Olas multi-chain pattern, mirrored on every EVM L2 / sidechain Olas governs (Polygon, Optimism, Gnosis, Celo, …) via the `WormholeMessenger.sol` contract pattern at `valory-xyz/autonolas-governance/contracts/bridges/`. The Solana version is the same architectural intent expressed in Solana idiom (inline VAA verification in a single program, rather than separate Messenger + Executor contracts as on EVM, because Solana's deploy economics and runtime norm prefers fewer / larger programs).

---

## 3. Repository state (the two work-streams)

The repository contains two parallel implementations of this concept:

| Branch | Program crate | Last code commit | Code volume | Bridge integration | PR | Deployed on mainnet |
|---|---|---|---|---|---|---|
| `main` | `fee_collector` | 2024-04-05 | 2 files, 242 LoC | None — 3 admin instructions are open to any signer; 3 `/// CHECK: Check later` markers | — (default branch) | No |
| `bridging4` | `lockbox_governor` | 2024-06-17 | 8 files, 1345 LoC | Full — `wormhole_anchor_sdk` posted-VAA reads; foreign-emitter + foreign-chain constraint checks; per-VAA replay defense via `Received` PDAs | **PR #1 OPEN since 2024-06-12** | No |

Mainnet operational reality:
- The `fee_collector` / `lockbox_governor` program at the declared ID `DWDGo2Uk...MYaB` is not deployed.
- `lockbox-solana`'s mainnet upgrade authority is the EOA `7mQQw6qCj66EtfarNKUfWGUD1pX7SSjHbVMpdJc1roK2`, held off-chain. The team chose direct-EOA-as-upgrade-authority instead of routing through the governance program.
- 21 months have passed (2024-09 → 2026-06) without a code-level decision between the two branches or on retirement.

Code-level differences (not exhaustive):
- `main`'s `fee_collector` has 4 instructions (`initialize`, `transfer`, `transfer_token_account`, `change_upgrade_authority`); `bridging4`'s `lockbox_governor` has 6 (`initialize`, `transfer`, `transfer_all`, `transfer_token_accounts`, `set_program_upgrade_authority`, `upgrade_program`).
- `main` accepts any signer on privileged ops; `bridging4` requires a posted VAA whose `emitter_chain` + `emitter_address` match values stored at init (`config.chain`, `config.foreign_emitter`).
- `main` has a `GovernorError` enum with 7 variants but uses none of them (dead code); `bridging4` uses 6 of the 7 (`InvalidForeignEmitter`, `InvalidForeignChain`, `WrongUpgradeAuthority`, `WrongTokenMint`, `WrongAccountOwner`, `WrongAccount`).
- `main` has `SOL` and `OLAS` Pubkey constants declared but unused; `bridging4` uses them in `transfer` / `transfer_all` / `transfer_token_accounts` to gate token mint.
- `main`'s state has `total_sol_transferred` + `total_olas_transferred` fields that are never written; `bridging4` increments them in `transfer` and `transfer_all`.

The pattern is consistent: `main` looks like an early prototype of `bridging4`, with the bridge integration and consequent validations stripped (or never added). The team's chronology (per git log: April 2024 main `feat: initial implementation`; June 2024 bridging4 `feat: finalizing initial governance code` + `feat: transfer all and upgrade program methods` + 10 audit-related commits) suggests `bridging4` was the intended production form and `main` is a stale early prototype that was never updated.

---

## 4. Findings on `main`

The audit on `main` produces the following items. F-1 / F-2 / F-3 are Critical-class design defects; F-4 through F-11 are Info-class hygiene / dead-code items.

### F-1 — `transfer` has NO access control (Critical-class design defect; dormant)

**Severity**: Critical (code); dormant per §3.
**Anchor**: `programs/fee_collector/src/lib.rs:47-67` + `lib.rs:135-149`.

```rust
pub struct TransferFeeCollector<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,                          // any signer
    pub collector_account: Box<Account<'info, TokenAccount>>,  // any token account
    pub destination: Box<Account<'info, TokenAccount>>,        // any destination
    /* ... */
}
```

`token::transfer` CPI is signed by the `collector` PDA via `&[&ctx.accounts.collector.seeds()]`. There is no constraint binding `signer` to a stored governance authority, no constraint on `collector_account.owner`, no constraint on `destination`. Any signer can construct `(collector_account = any-PDA-owned-token-account, destination = their-own)` and drain.

Why dormant in production: the program isn't deployed; if it were, no SPL token accounts would be assigned to its PDA (assignment requires `transfer_token_account` to be called first, which has the same defect).

### F-2 — `transfer_token_account` has NO access control + Unchecked destination

**Severity**: Critical (code); dormant.
**Anchor**: `lib.rs:70-93` + `lib.rs:152-167`.

```rust
pub struct TransferTokenAccountFeeCollector<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,                          // any signer
    pub collector_account: Box<Account<'info, TokenAccount>>,  // any token account

    /// CHECK: Check later                              // marker: validation deferred, never landed
    #[account(mut)]
    pub destination: UncheckedAccount<'info>,           // any pubkey, not even a TokenAccount
    /* ... */
}
```

The CPI is `set_authority(AccountOwner, destination)` signed by the `collector` PDA. Any signer can call this with any PDA-owned token account as `collector_account` and any pubkey as `destination`, transferring ownership of the token account away from the program.

### F-3 — `change_upgrade_authority` has NO access control + 3 Unchecked accounts

**Severity**: Critical (code); dormant.
**Anchor**: `lib.rs:96-117` + `lib.rs:170-188`.

```rust
pub struct ChangeUpgradeAuthorityFeeCollector<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,                          // any signer

    /// CHECK: Check later                              // never honored
    #[account(mut)]
    pub program_to_update_authority: UncheckedAccount<'info>,

    /// CHECK: Check later                              // never honored
    #[account(mut)]
    pub program_data_to_update_authority: UncheckedAccount<'info>,

    /// CHECK: Check later                              // never honored
    #[account(mut)]
    pub destination: UncheckedAccount<'info>,
    /* ... */
}
```

The CPI is `set_upgrade_authority(program, current_authority=PDA, new_authority=destination)` signed by the PDA. Three Unchecked accounts with literal «CHECK: Check later» comments. Any signer can transfer upgrade authority of any program whose current upgrade authority is this PDA to any destination.

### F-4 — `total_sol_transferred` / `total_olas_transferred` never updated

**Severity**: Info.
**Anchor**: `state.rs:6-9` (declared) + body of `transfer` (does not write).

These state fields exist (and are zero-initialized at `initialize`) but no instruction ever increments them. They are dead state on this branch. (On `bridging4` they are used.)

### F-5 — `TransferEvent` declared but never emitted

**Severity**: Info.
**Anchor**: `lib.rs:196-204`.

Event struct defined; no `emit!` call anywhere. Dead code on this branch. (On `bridging4` the event is emitted from `transfer`.)

### F-6 — `ErrorCode::WrongTokenMint` declared but never thrown

**Severity**: Info.
**Anchor**: `lib.rs:190-194`.

Single error-enum variant; never returned. (On `bridging4` this is one of 6 used variants of a 7-variant `GovernorError` enum.)

### F-7 — Unused imports

**Severity**: Info.
**Anchor**: `lib.rs:3-12`.

`anchor_spl::token::{Mint, Approve}`, `bpf_loader_upgradeable::upgrade`, `loader_upgradeable_instruction::UpgradeableLoaderInstruction`, `spl_token::instruction::transfer` are imported but never referenced. `cargo build` emits warnings.

### F-8 — Toolchain stale; `cargo audit` flags transitive dev-time vulnerabilities

**Severity**: Info (runtime exposure = 0 since on-chain Solana program binary does not include these crates).
**Source**: `cargo audit` against `Cargo.lock`.

Same situation as the sister `lockbox-solana` audit. Anchor 0.26.0 / Solana 1.14.29 / Rust 1.62 are pinned at mid-2022 versions. `cargo audit` flags `curve25519-dalek 3.2.1` (RUSTSEC-2024-0344 timing), `ed25519-dalek 1.0.1` (RUSTSEC-2022-0093 oracle attack), 8 unmaintained-crate warnings. None of these are in the compiled program binary. Refresh clears them whenever Anchor / Solana / Orca tooling matures.

### F-9 — Mainnet bytecode-vs-source verification (recommended follow-up)

**Severity**: Info.

Verified 2026-06-03 via RPC `getAccountInfo`:
- This program (`DWDGo2Uk...MYaB`): not deployed on mainnet.
- `lockbox-solana` mainnet (`1BoXeb8...jY3`) ProgramData PDA (`4db7h3Hi...8KXc`) upgrade authority: `7mQQw6q...`, a System-owned EOA (not a PDA, not a program). Single-key off-chain control.

These two facts together establish the «dormant in production» qualifier on F-1 / F-2 / F-3.

### F-10 — `initialize` has no admin check (first caller wins implicit ownership)

**Severity**: Info.
**Anchor**: `lib.rs:30-44` + `lib.rs:121-133`.

`init` ensures one-shot creation (re-init impossible), but the first signer to call `initialize` wins. No `governance_authority` field is stored anywhere, so even if F-1/F-2/F-3 were patched with a stored-admin gate, there'd be nothing in state to compare against. (On `bridging4`, init stores `chain` and `foreign_emitter`, which are then the trust anchors.)

### F-11 — `SOL` and `OLAS` constants declared but never enforced

**Severity**: Info.
**Anchor**: `lib.rs:21-23`.

Two Pubkey constants exist but neither is referenced in instruction validation. The implication «this program only handles SOL/OLAS» is documentation-only; the program accepts any SPL mint. (On `bridging4`, both constants ARE enforced in `transfer` / `transfer_all` / `transfer_token_accounts`.)

---

## 5. Findings on `bridging4` PR #1

The audit on `bridging4` produces the following items. F-B-2 is High-class (explicit acknowledged TODO); F-B-1 is Medium structural; F-B-3 is Low; F-B-4 through F-B-11 are Info.

### F-B-1 — `wormhole_program` is `UncheckedAccount`; Wormhole Core program identity relies on implicit Anchor SDK behavior

**Severity**: Medium (defense-in-depth; current SDK behavior makes it non-exploitable; brittle to SDK changes).
**Anchor**: `programs/lockbox_governor/src/context.rs:60-71` + four analogous places (one per privileged instruction):

```rust
/// CHECK: Wormhole program account. Required for posted account check. Read-only.
pub wormhole_program: UncheckedAccount<'info>,

#[account(
    seeds = [wormhole::SEED_PREFIX_POSTED_VAA, &vaa_hash],
    bump,
    seeds::program = wormhole_program           // ← caller-supplied program
)]
pub posted: Box<Account<'info, wormhole::PostedVaa<TransferMessage>>>,
```

The `wormhole_program` field has no `address = WORMHOLE_CORE_PROGRAM_ID` constraint and no Config-stored reference. The `posted` PDA is derived using `seeds::program = wormhole_program`, i.e., the caller's value.

Why this is current-SDK-safe: Anchor's `Box<Account<'info, PostedVaa<P>>>` enforces `posted.owner == PostedVaa::owner()`, which `wormhole_anchor_sdk` declares as the Wormhole Core program ID per network. The intersection «PDA derivable from `wormhole_program` AND owned by Wormhole Core» forces `wormhole_program == Wormhole Core` in practice.

Why this is brittle: the safety property depends on the SDK's owner-trait implementation. A future SDK refactor that changes how `PostedVaa::owner()` is resolved (e.g., dynamic owner) breaks the implicit constraint without changing this code's surface.

**Recommendation**: add an explicit constraint binding `wormhole_program` to the right program. Either store the Wormhole Core program ID at `initialize` time in `Config.wormhole.bridge` (currently declared but never set — see F-B-4) and constrain via `address = config.wormhole.bridge`, or hardcode the program ID as a module-level constant. Either makes the audit a single line of trust rather than «trust the SDK + hope derivation behaves».

### F-B-2 — `// TODO: verifications` comment in `transfer` instruction body

**Severity**: High (acknowledged gap of unknown scope).
**Anchor**: `programs/lockbox_governor/src/lib.rs:120`.

Inside the body of the `transfer` instruction, after the source/destination/mint checks and before the `token::transfer` CPI:

```rust
// TODO: verifications
```

The developer left an explicit TODO before merge. There is no associated issue, PR comment, or design doc that specifies what additional verification was planned. Without the author's clarification, the severity of the missing check cannot be determined — it could range from a redundant defense-in-depth check (Info) to a critical bypass (Critical).

**Recommendation**: resolve before merge. Either land the planned check (with a comment explaining what it defends against) or close the TODO with a comment explaining why no additional check is needed at this site.

### F-B-3 — `Received` PDAs grow monotonically; no `close` instruction

**Severity**: Low (operational; sealed state grows linearly with VAA count).
**Anchor**: `context.rs:76-87` (Received init for `transfer`) + four analogous places + absence of any close instruction in `lib.rs`.

Each privileged instruction creates a new `Received` PDA via `init` (payer = signer). The PDA is keyed by `(SEED_PREFIX, emitter_chain, sequence)`, providing replay defense. There is no `close_received` instruction to reclaim rent.

For a low-volume governance protocol (≤ ~100 messages/year), this is modest (~0.001 SOL rent per PDA, ~0.1 SOL/year locked up). For higher volume it grows linearly without bound.

**Recommendation**: add a `close_received` instruction gated by the same VAA-emitter checks, or accept the growth and document it.

### F-B-4 — `Config.wormhole` (`WormholeAddresses`) declared but never initialized

**Severity**: Info (dead state until F-B-1 fix lands).
**Anchor**: `state/config.rs:5-20` (struct) + `lib.rs:43-75` (`initialize` body).

```rust
pub struct Config {
    pub wormhole: WormholeAddresses,    // 3 × Pubkey = bridge + fee_collector + sequence
    /* ... */
}
```

`initialize` sets `finality`, `bump`, `chain`, `foreign_emitter`, and the two `total_*_transferred` counters. It never writes `config.wormhole`, so the 96 bytes stay at `Pubkey::default()` zeros.

**Recommendation**: either populate at init (and use `config.wormhole.bridge` to fix F-B-1) or remove the field.

### F-B-5 — `transfer_token_accounts` destination is Unchecked

**Severity**: Info (governance trust assumption).
**Anchor**: `context.rs:255-258`.

```rust
/// CHECK: This is any account provided by the governor.
#[account(mut)]
pub destination_account: UncheckedAccount<'info>,
```

The relayed VAA's `destination` payload is passed to SPL Token `set_authority(AccountOwner, destination)`. No defensive check that `destination` is on-curve, not the System Program, not equal to a known no-spend address.

**Recommendation**: defensive check that destination is not the System Program ID at minimum. The L1 Timelock's correctness is the primary guarantee; this is foot-gun protection.

### F-B-6 — `Pubkey::try_from(*source).unwrap()` is safe

**Severity**: N/A (initial concern resolved on review).
**Anchor**: `lib.rs:96, 162-165, 233-235, 290-291, 323-325`.

`Pubkey::try_from(&[u8; 32])` is infallible because the input is a fixed-size 32-byte array (the exact Pubkey internal representation). `.unwrap()` is safe; cosmetic improvement: `.expect("infallible")` would document the invariant.

### F-B-7 — Unchecked `+=` on u64 totals

**Severity**: Info.
**Anchor**: `lib.rs:105-109` (and analogous in `transfer_all`).

```rust
ctx.accounts.config.total_sol_transferred += *amount;
ctx.accounts.config.total_olas_transferred += *amount;
```

Not `checked_add`. With `[profile.release] overflow-checks = true` (Cargo.toml line 17), arithmetic overflow panics → transaction reverts cleanly. Functionally safe; `checked_add` would be more explicit.

### F-B-8 — `initialize` has no admin check (first-caller wins implicit init authority)

**Severity**: Info (deployment-trust note, same family as F-10 on `main`).
**Anchor**: `lib.rs:43-75` + `context.rs:23-43`.

`initialize(chain, timelock)` accepts any signer; the first caller chooses the trusted L1 governance source forever (no `change_*` instruction for `chain` or `foreign_emitter` exists).

**Recommendation**: deploy script must be the legitimate caller; treat `(chain, timelock)` as a deployment-time commitment.

### F-B-9 — No `change_foreign_emitter` mechanism

**Severity**: Info (architectural note).

If the Olas L1 Timelock address ever changes (Timelock upgrade on Ethereum, governance migration), there is no way to update `config.foreign_emitter` on the Solana side. Would require redeploying the entire `lockbox_governor` program and migrating upgrade authority + token accounts to the new PDA.

EVM `WormholeMessenger.sol` has a `changeSourceGovernor` function gated by self-call (only a currently-trusted-Timelock-relayed VAA can change to a new Timelock). Adding the same pattern on Solana would mirror the EVM design.

### F-B-10 — VAA sequence ordering not enforced

**Severity**: Info (governance-use-case dependent).

Each `Received` PDA is keyed by `(emitter_chain, sequence)` so each VAA is processed at most once. There is no enforcement that sequences are processed in monotonic order — sequence 5 could be processed before sequence 3. For commutative governance actions this is fine; for ordering-sensitive sequences (e.g., «change upgrade authority then upgrade») the L1 Timelock must not emit such patterns.

**Recommendation**: document the «governance must not emit ordering-sensitive sequences» constraint. If ordering enforcement is later needed, add `last_processed_sequence` to Config and require `sequence == last_processed_sequence + 1`.

### F-B-11 — `UpgradeProgramEvent` does not include buffer-content hash

**Severity**: Info (audit-trail completeness).
**Anchor**: `events.rs:60-75`.

The event records buffer / program / spill addresses but not a hash of the bytecode being deployed. Reconstructing «which bytecode was deployed at this VAA» from on-chain history requires reading the (post-deploy-freed) buffer account's content. A `buffer_content_hash: [u8; 32]` field would close the audit-trail gap.

### Findings summary on `bridging4`

| # | Severity | Item |
|---|---|---|
| F-B-1 | Medium | Wormhole Core program identity is implicit via Anchor `PostedVaa` owner trait; brittle |
| F-B-2 | **High** | `// TODO: verifications` comment in `transfer` body — explicit acknowledged gap of unknown scope |
| F-B-3 | Low | `Received` PDAs grow monotonically; no `close` instruction |
| F-B-4 | Info | `Config.wormhole` field declared but never initialized |
| F-B-5 | Info | `transfer_token_accounts` destination is Unchecked, no defensive validation |
| F-B-6 | N/A | `Pubkey::try_from(.).unwrap()` safe — false alarm |
| F-B-7 | Info | Unchecked `+=` on u64 totals (overflow-checks=true makes safe) |
| F-B-8 | Info | `initialize` no admin gate (deployment-trust note) |
| F-B-9 | Info | No `change_foreign_emitter` mechanism |
| F-B-10 | Info | VAA sequence ordering not enforced |
| F-B-11 | Info | `UpgradeProgramEvent` does not include buffer-content hash |

**Critical or fund-loss-class on `bridging4`: none.** The bridge-origin gates (the three checks per privileged instruction) ARE correctly implemented. The 1 High (F-B-2) is unknown-scope; the 1 Medium (F-B-1) is current-SDK-safe. Remaining items are Info/Low operational.

---

## 6. Anchor security pattern checklist

| # | Pattern | `main` (`fee_collector`) | `bridging4` (`lockbox_governor`) |
|---|---|---|---|
| A-01 | PDA seed verification (`seeds = true`) | ✓ | ✓ |
| A-02 | Signer enforcement | ⚠ NO admin gate — F-1/2/3 | ✓ Gated on posted-VAA origin |
| A-03 | `init` PDA space matches struct size | ✓ | ✓ |
| A-04 | `init_if_needed` careful usage | ✓ (not used) | ✓ (not used) |
| A-05 | `realloc` safety | ✓ N/A | ✓ N/A |
| A-06 | CPI program-ID validation | ⚠ — F-3 | ✓ (partial; F-B-1 caveat on Wormhole Core ID) |
| A-07 | Owner / discriminator checks | ⚠ — UncheckedAccount + «CHECK: Check later» | ✓ Anchor-enforced; F-B-5 minor |
| A-08 | Account mutability | ✓ | ✓ |
| A-09 | Overflow checks (`overflow-checks = true`) | ✓ | ✓ (F-B-7 uses unchecked +=) |
| A-10 | Reentrancy | ✓ N/A (Solana runtime) | ✓ N/A |
| A-11 | Closing / lamports drain | ✓ N/A | ⚠ F-B-3 (no close, PDAs grow) |
| A-12 | Token authority transfer correctness | ❌ F-2 | ✓ Gated on VAA |
| A-13 | Sysvars usage | ✓ | ✓ |
| A-14 | `declare_id!` vs deployed | ✓ (matches Anchor.toml; not deployed mainnet) | ✓ |
| A-15 | Bump persistence | ✓ | ✓ |
| A-16 | UncheckedAccount with justified `/// CHECK:` | ❌ — «Check later» × 3 | ⚠ (some UncheckedAccount with one-line justification; F-B-5 minor) |
| A-17 | Privileged-instruction admin gating | ❌ — F-1/2/3 | ✓ |
| A-18 | Constants used vs declared | ⚠ F-11 (SOL/OLAS unused) | ✓ |

---

## 7. Adversarial sweep — what's NOT in the dev's claims

1. **README claims «Timelock governed»; `main`'s code does not implement any gate.** Documentation-vs-code mismatch on the default branch. (`bridging4`'s code does implement the gate; the README on `bridging4` is the same as on `main`.)
2. **«CHECK: Check later» markers in `main` are explicit author-time TODOs that were never returned to.** A pre-merge code review discipline that catches `/// CHECK: Check later` should have blocked merge to main.
3. **PR #1 has been open for ≈2 years without merge.** The architectural-debt is at the project process layer (review-and-merge), not at the engineering layer (the implementation exists).
4. **Mainnet upgrade authority is an EOA, not a multisig.** `7mQQw6q...` is a System-owned single-key account. Whoever holds the private key for `7mQQw6q...` can deploy any code as the next `lockbox-solana` upgrade and drain all locked liquidity. This is a centralization risk separate from this audit's scope; it lives at the project-architecture level.

---

## 8. Compliance report

| Item | Status |
|---|---|
| All `.rs` files read across both branches (242 + 1345 LoC) | ✓ |
| Anchor + Solana + cross-domain pattern checklist applied to both | ✓ (§6) |
| Branch / open-PR enumeration at scope-inventory | ✓ |
| `cargo audit` run | ✓ (F-8) |
| README claims verified against actual code (VCE discipline) | ✓ — `main`'s «Timelock governed» does not hold; `bridging4`'s does (modulo F-B-1's brittleness) |
| On-chain mainnet state checked via RPC | ✓ — program not deployed; lockbox-solana upgrade authority is EOA |
| Adversarial sweep | ✓ (§7) |
| Forward-look frames the project-level decision | ✓ (§9) |

---

## 9. Forward-look

The team has three coherent paths.

### Path A — Merge PR #1 after addressing the findings on `bridging4`

Apply fixes for F-B-1 (bind `wormhole_program` to an explicit Wormhole Core program ID, ideally stored at init in `Config.wormhole.bridge`), F-B-2 (resolve or close the `// TODO: verifications`), F-B-3 (decide on `close_received` instruction or document growth). F-B-5 through F-B-11 are Info/Low and can land as a separate hygiene pass.

After fixes, replace `main`'s `fee_collector` with `bridging4`'s `lockbox_governor` (the `main`'s F-1/F-2/F-3 broken prototype is then gone). Re-audit the merged result.

Mainnet rollout requires a coordinated step: the L1 Olas Timelock must emit the first VAA transferring `lockbox-solana`'s upgrade authority from the current `7mQQw6q...` EOA to the deployed `lockbox_governor` PDA. This first step is initiated by whoever holds the `7mQQw6q...` private key (chicken-and-egg: can't be relayed via the governor since the governor isn't yet wired in).

### Path B — Abandon PR #1 and add a simple Solana-native admin gate to `main`

If for some reason the team has decided that bridge integration is not pursued, the simpler path is to add `governance_authority: Pubkey` to `FeeCollector` state at init, gate every privileged instruction by `signer.key() == &collector.governance_authority`, address F-4 through F-11 hygiene items, and rewrite the README to say «governed by Olas team multisig at address X», not «Timelock governed».

This is a coherent design but deviates from the Olas multi-chain pattern (every other Olas-governed chain uses the L1-Timelock-via-bridge model). Either the team has a reason for the Solana deviation, or this path is suboptimal relative to Path A.

### Path C — Formally retire the program

Close PR #1 with rationale, mark the README as deprecated, archive the repo on GitHub. Document that `lockbox-solana`'s upgrade authority remains the off-chain `7mQQw6q...` EOA and that Olas Solana governance is «Olas team multisig (off-chain) + dispatch-by-discipline timelock» — a centralization disclosure but at least honest.

The separate concern that `7mQQw6q...` is a single-key EOA rather than a multisig is outside this audit's scope; if Path C is chosen, the team should also consider transferring upgrade authority from the EOA to a Squads multisig as a separate operational improvement.

### Recommended path

**Path A is the architecturally consistent answer** — it preserves «Olas governance is unified across all chains via the Ethereum Timelock», matches what every other Olas-governed chain does, and means the in-flight PR #1 work is not wasted. The two structural fixes on `bridging4` (F-B-1, F-B-2) are well-scoped; the rest of the audit findings are Info/Low operational items that don't block merge.

**Either way, do not deploy `main`'s `fee_collector` as-is.** It is the broken prototype and its three Critical-class defects have no defensible production path.

---

## 10. Cross-references

- `main` source under audit: `programs/fee_collector/` at HEAD `735802d` (2026-05-25)
- `bridging4` source under audit: `programs/lockbox_governor/` at HEAD `dd901fb` (2024-06-17; PR #1 «Featuring EVM-Solana governor implementation», open since 2024-06-12)
- Declared mainnet program ID: `DWDGo2UkBUFZ3VitBfWRBMvRnHr7E2DSh57NK27xMYaB` — not deployed (verified 2026-06-03)
- Mainnet `lockbox-solana` (`1BoXeb8...jY3`) upgrade authority: `7mQQw6qCj66EtfarNKUfWGUD1pX7SSjHbVMpdJc1roK2` (System-owned EOA; verified 2026-06-03)
- Olas multi-chain governance pattern reference: `valory-xyz/autonolas-governance/contracts/bridges/WormholeMessenger.sol` — the canonical L2-Executor implementation on EVM L2 / sidechain Olas-governed deployments
- Wormhole Core program ID on Solana mainnet: `worm2ZoG2kUd4vFXhvjh93UUH596ayRfgQ2MgjNMTth`
- Sister Solana audits in this cycle: `lockbox-solana` (`audits/internal2/` 2026-06-02), `registries-solana` (`audits/internal1/` 2026-06-02)
- Prior `bridging4` internal audit folder: `audits/internal/` on the same branch (2024-06; not reviewed in detail as part of this audit; recommend cross-referencing against F-B-1 / F-B-2 / F-B-3 before PR #1 merge)

---

*audit-claude, 77ph, 2026-06-03*
