#!/bin/bash

# Clean everything
# rustup self uninstall
# rm -rf ~/.local/share/solana && rm -rf ~/.cache/solana && rm -rf ~/.cargo && rm -rf ~/.avm

RUSTVER="1.79"
SOLANAVER="1.18.22"
ANCHORVER="0.30.1"

# Quick change of solana version:
#solana-install init $SOLANAVER

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup install $RUSTVER
rustup default $RUSTVER

sh -c "$(curl -sSfL https://release.anza.xyz/v${SOLANAVER}/install)"
#curl -sSfL https://release.solana.com/v${SOLANAVER}/install | sh

cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
avm install $ANCHORVER
avm use $ANCHORVER

