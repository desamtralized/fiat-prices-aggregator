# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Development Commands

### Build Commands
```bash
# Build the project in release mode
cargo build --release

# Build for development
cargo build

# Run clippy for linting
cargo clippy

# Run tests
cargo test

# Run a specific test
cargo test test_name
```

### Deployment Setup
```bash
# Setup cron job (runs every 15 minutes)
sh cron.sh
# This will output: */15 * * * * $PWD/target/release/local-money-fiat-price-aggregator >> /var/log/local-price.log 2>&1
```

## Architecture Overview

This is a Rust-based fiat price aggregator for the Local Money protocol on the Kujira blockchain. The system fetches USD exchange rates for various fiat currencies and stores them on-chain.

### Core Components

1. **Main Orchestrator** (`src/main.rs`): 
   - Fetches prices from Yadio API
   - Derives wallet from mnemonic seed
   - Constructs and broadcasts CosmWasm transaction to update prices on-chain
   - Handles currencies: ARS, BRL, CAD, CLP, COP, EUR, GBP, MXN, NGN, THB, VES

2. **API Module** (`src/api/`):
   - Primary source: `yadio.rs` - fetches all currency prices from api.yadio.io
   - Alternative sources available: binance, buda, cripto_ya, mercado_bitcoin
   - Uses failover pattern with default values if API fails

3. **Transaction Building**:
   - Uses cosmrs library for Cosmos SDK transactions
   - Constructs MsgExecuteContract messages for the price contract
   - Fixed gas: 300,000 units at 358 ukuji

### Environment Configuration

Required `.env` variables:
- `ADMIN_SEED`: Mnemonic seed phrase for admin wallet
- `PRICE_ADDR`: Address of the price contract on Kujira
- `CHAIN_ID`: Kujira chain ID (e.g., "harpoon-4" for testnet)
- `LCD`: LCD endpoint URL
- `RPC`: RPC endpoint URL
- `ADDR_PREFIX`: Address prefix ("kujira")

### Key Dependencies
- `cosmrs`: Cosmos SDK transaction handling
- `localterra-protocol`: Protocol types from GitHub repository
- `bip39`/`bip32`: Wallet derivation from mnemonic