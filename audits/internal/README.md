# Internal audit of lockbox-governor-solana
The review has been performed based on the contract code in the following repository:<br>
`https://github.com/valory-xyz/lockbox-governor-solana` <br>
commit: `7a1381a0885a0fad49be24225d6a483c54701a76` or `v0.1.0-pre-internal-audit`<br> 

## Objectives
The audit focused on contracts in folder `programs`.

### Flatten version
N/A

### OS Requirments checks
Pre-requisites (README.md)
```
anchor --version
anchor-cli 0.29.0
solana --version
solana-cli 1.18.1 (src:5d824a36; feat:756280933, client:SolanaLabs)
rustc --version
rustc 1.74.1 (a28077b28 2023-12-04)
```
Checks - passed [-]
```
bash setup-env.sh 
anchor --version
anchor-cli 0.29.0
solana --version
solana-cli 1.18.5 (src:928d8ac2; feat:3352961542, client:SolanaLabs)
rustc --version
rustc 1.78.0 (9b00956e5 2024-04-29)
```
Issue: The versions written in the readme do not match in detail the ones in the script.
```
In setup-env.sh
RUSTVER="1.78"
SOLANAVER="1.18.5"
ANCHORVER="0.29.0"
```

## Security issues.
### Problems found instrumentally
Several checks are obtained automatically. They are commented. Some issues found need to be fixed. <br>
List of rust tools:
##### cargo tree
```
cargo tree > audits/internal/analysis/cargo_tree.txt
```
##### cargo-audit
https://docs.rs/cargo-audit/latest/cargo_audit/
```
cargo install cargo-audit
cargo-audit audit > audits/internal/analysis/cargo-audit.txt
```
##### cargo clippy 
https://github.com/rust-lang/rust-clippy
```
cargo clippy 2> audits/internal/analysis/cargo-clippy.txt
```
##### cargo-spellcheck
https://github.com/drahnr/cargo-spellcheck
```
sudo apt install llvm
llvm-config --prefix 
/usr/lib/llvm-14
sudo apt-get install libclang-dev
cargo install --locked cargo-spellcheck
cd programs/lockbox_governor
cargo spellcheck -r list-files
cargo spellcheck --verbose check
```
All automatic warnings are listed in the following file, concerns of which we address in more detail below: <br>
[cargo-tree.txt](https://github.com/valory-xyz/lockbox-governor-solana/blob/main/lockbox/audits/internal/analysis/cargo-tree.txt) <br>
[cargo-audit.txt](https://github.com/valory-xyz/lockbox-governor-solana/blob/main/lockbox/audits/internal/analysis/cargo-audit.txt) <br>
[cargo-clippy.txt](https://github.com/valory-xyz/lockbox-governor-solana/blob/main/lockbox/audits/internal/analysis/cargo-clippy.txt) <br>
[cargo-spellcheck.txt](https://github.com/valory-xyz/lockbox-governor-solana/blob/main/lockbox/audits/internal/analysis/cargo-spellcheck.txt) <br>
Notes: <br>
https://rustsec.org/advisories/RUSTSEC-2022-0093 - out of scope

### Problems found by manual analysis 14.06.24
#### Specific bridge repeat attack
```
It is not clear at the code level how it is protected from re-use of the same posted/vaa.

Exists as example:
ctx.accounts.received.batch_id = posted_message.batch_id();
ctx.accounts.received.wormhole_message_hash = vaa_hash;
ctx.accounts.received.sequence = posted_message.sequence();
But they never participate in any checking. (?)

Expected logical result: a message coming from L1 can be processed on Solana only once for every each unique message.
I.e. 
L1 -> msg1(transfer,source,destination,amount) -> Solana
on Solana -> run msg1(transfer,source,destination,amount) - OK
on Sonala -> run msg1(transfer,source,destination,amount) - expected Fail
```
To discussion. <br>

#### List of common attack vectors
https://www.sec3.dev/blog/how-to-audit-solana-smart-contracts-part-1-a-systematic-approach <br>
https://medium.com/@zokyo.io/what-hackers-look-for-in-a-solana-smart-contract-17ec02b69fb6 <br>
1. Missing signer checks (e.g., by checking AccountInfo::is_signer ) <br>
N/A

2. Missing ownership checks (e.g., by checking  AccountInfo::owner) <br>
```rust
pub config: Box<Account<'info, Config>>
pub posted: Box<Account<'info, wormhole::PostedVaa<TransferMessage>>>
pub program_account: UncheckedAccount<'info> (to discussion)
pub buffer_account: UncheckedAccount<'info> (to discussion)
```
3. Missing rent exemption checks <br>
in `upgrade_program`? <br>
To discussion. <br>

4. Signed invocation of unverified programs <br>
```rust
// Wormhole program.
/// CHECK: Wormhole program account. Required for posted account check. Read-only.
pub wormhole_program: UncheckedAccount<'info>,
Not checked by address?

pub fn transfer() - OK
pub fn transfer_all() - OK
pub fn transfer_token_accounts() - OK
pub fn set_program_upgrade_authority() - OK
pub fn upgrade_program() - OK
```
To discussion. <br>

5. Solana account confusions: the program fails to ensure that the account data has the type it expects to have. <br>
```rust
program_account vs program_data_account
https://github.com/solana-labs/solana/blob/master/sdk/program/src/bpf_loader_upgradeable.rs#L29
Buffer {
        /// Authority address
        authority_address: Option<Pubkey>,
        // The raw program data follows this serialized structure in the
        // account's data.
    },
    /// An Program account.
    Program {
        /// Address of the ProgramData account.
        programdata_address: Pubkey,
    },
    // A ProgramData account.
    ProgramData {
        /// Slot that the program was last modified.
        slot: u64,
        /// Address of the Program's upgrade authority.
        upgrade_authority_address: Option<Pubkey>,
        // The raw program data follows this serialized structure in the
        // account's data.
```

6. Re-initiation with cross-instance confusion <br>
Passed. Example: https://github.com/coral-xyz/sealevel-attacks/blob/master/programs/4-initialization/recommended/src/lib.rs

7. Arithmetic overflow/underflows: If an arithmetic operation results in a higher or lower value, the value will wrap around with two’s complement. <br>
```rust
ctx.accounts.config.total_sol_transferred += amount_sol;
ctx.accounts.config.total_olas_transferred += amount_olas;
```
To discussion. <br>

8. Numerical precision errors: numeric calculations on floating point can cause precision errors and those errors can accumulate. <br>
N/A

9. Common issue <br>
```
checking source_account != destination_account
from: ctx.accounts.source_account.to_account_info(),
to: ctx.accounts.destination_account.to_account_info(),
```


Notes:
1. Remove msg! in production version.
2. `// TODO: verifications` ? amount > 0?
