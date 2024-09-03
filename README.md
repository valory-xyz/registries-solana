# Registries Solana
Set of Autonolas registries contracts on Solana.

## Pre-requisites
The program requires that the following environment is satisfied:
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

To run the initial script that would just initialize the registries program run:
```
solana airdrop 10000 9fit3w7t6FHATDaZWotpWqN7NpqgL3Lm1hqUop4hAy8h --url localhost && npx ts-node tests/init.ts
```

To run integration tests, make sure to stop and start the `validator.sh` in a separate window. Then run:
```
solana airdrop 10000 9fit3w7t6FHATDaZWotpWqN7NpqgL3Lm1hqUop4hAy8h --url localhost && npx ts-node tests/registries.ts
```

The deployed program ID must be `7J1mLX2ozMwU6p6mX7zuXMoZf5SBwLBZrGevJHpXP98k` and corresponds to the `declare_id`
in the `programs/registires/src/lib.rs` and `Anchor.toml` file.

For debugging a program address, after the launch of local validator, run:
```
solana logs -v --url localhost 7J1mLX2ozMwU6p6mX7zuXMoZf5SBwLBZrGevJHpXP98k
```
