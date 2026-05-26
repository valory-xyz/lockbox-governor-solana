#!/bin/bash

# Clean everything
# rustup self uninstall
# rm -rf ../../.local/share/solana && rm -rf ../../.cache/solana && rm -rf ../../.cargo && rm -rf ../../.avm

RUSTVER="1.78"
SOLANAVER="1.18.5"
ANCHORVER="0.29.0"

# Quick change of solana version:
#solana-install init $SOLANAVER

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup install $RUSTVER
rustup default $RUSTVER

curl -sSfL https://release.solana.com/v${SOLANAVER}/install | sh

cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
avm install $ANCHORVER
avm use $AN CHORVER

