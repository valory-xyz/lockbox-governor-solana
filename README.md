# Lockbox Governor Solana
Timelock governed OLAS and SOL accounts Fee Collector and Program Manager on Solana.

## Pre-requisites
The programs requires that the following environment is satisfied:
```
anchor --version
anchor-cli 0.29.0
solana --version
solana-cli 1.18.1 (src:5d824a36; feat:756280933, client:SolanaLabs)
rustc --version
rustc 1.74.1 (a28077b28 2023-12-04)
```

Advise the script `setup-env.sh` to correctly install the required environment.

## Development
Install the dependencies:
```
yarn
```

If you need to remove / check dependencies, run:
```
cargo clean
cargo tree
```

You might also want to completely remove the `Cargo.lock` file.

Build the code with:
```
anchor build
```

Run the validator in a separate window:
```
./validator.sh
```

Export environment variables:
```
export ANCHOR_PROVIDER_URL=http://127.0.0.1:8899
export ANCHOR_WALLET=artifacts/id.json
```

To run the initial script that would just initialize the lockbox governor program run:
```
solana airdrop 10000 9fit3w7t6FHATDaZWotpWqN7NpqgL3Lm1hqUop4hAy8h --url localhost && npx ts-node tests/lockbox_governor_init.ts
```

To run integration tests, make sure to stop and start the `validator.sh` in a separate window. Then run:
```
solana airdrop 10000 9fit3w7t6FHATDaZWotpWqN7NpqgL3Lm1hqUop4hAy8h --url localhost && npx ts-node tests/lockbox_governor_transfer_sol.ts
solana airdrop 10000 9fit3w7t6FHATDaZWotpWqN7NpqgL3Lm1hqUop4hAy8h --url localhost && npx ts-node tests/lockbox_governor_transfer_all.ts
solana airdrop 10000 9fit3w7t6FHATDaZWotpWqN7NpqgL3Lm1hqUop4hAy8h --url localhost && npx ts-node tests/lockbox_governor_transfer_token_accounts.ts
solana airdrop 10000 9fit3w7t6FHATDaZWotpWqN7NpqgL3Lm1hqUop4hAy8h --url localhost && npx ts-node tests/lockbox_governor_set_upgrade_authority.ts
solana airdrop 10000 9fit3w7t6FHATDaZWotpWqN7NpqgL3Lm1hqUop4hAy8h --url localhost && npx ts-node tests/lockbox_governor_upgrade_program.ts
```

Note that last two tests have to be executed semi-manually, read instructions in each of the test.

The deployed program ID must be `DWDGo2UkBUFZ3VitBfWRBMvRnHr7E2DSh57NK27xMYaB` and corresponds to the `declare_id`
in the `programs/lockbox_governor/src/lib.rs` and `Anchor.toml` file.

For debugging a program address, after the launch of local validator, run:
```
solana logs -v --url localhost DWDGo2UkBUFZ3VitBfWRBMvRnHr7E2DSh57NK27xMYaB
```

## Acknowledgements
The contracts were based on and inspired by the following sources:
- [Wormhole Docs](https://docs.wormhole.com/);
- [Wormhole Foundation](https://github.com/wormhole-foundation/wormhole-scaffolding).
