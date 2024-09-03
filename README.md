# Registries Solana
Set of Autonolas registries contracts on Solana.

## Pre-requisites
The program requires that the following environment is satisfied:
```
anchor --version
anchor-cli 0.30.1
solana --version
solana-cli 2.0.8 (src:3e7563cd; feat:1420694968, client:Agave)
rustc --version
rustc 1.79.0 (129f3b996 2024-06-10)
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
