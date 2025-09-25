# ZK DeFI Indexer

ZK DeFi Indexer for Uniswap v3 on Arbitrum (Nitro), written in Rust. The indexer ingests Arbitrum batch events from L1 Ethereum, decodes/decompresses Nitro batches, extracts L2 transactions, filters/decodes Uniswap v3 interactions, and exposes data via an API (planned). This repository currently contains a working Alloy-based WebSocket log subscriber for the Arbitrum Inbox/Sequencer contract and a concrete roadmap to a full indexer.

## Key goals

- Parse Arbitrum L1 batch events, decompress (Brotli), and extract L2 transactions
- Detect and decode Uniswap v3 pool/router calls (swap, mint, burn)
- Normalize and store events (Postgres as the canonical store; optional Redis/RocksDB cache)
- Serve data via Axum HTTP API; later power a simple Next.js explorer
- Handle reorgs and idempotent ingestion

## Current status (MVP slice implemented)

- `indexer-rs/src/main.rs` connects to an Ethereum WebSocket endpoint (e.g., Alchemy/Infura) via Alloy and subscribes to logs for a configured Arbitrum contract address. Incoming logs are printed for development and validation.
- This provides the live L1 event feed that will later be decoded into Nitro batches.

Next steps (see Roadmap) add batch decoding, Uniswap ABI parsing, persistence, and Axum API.

## Repository layout

- `indexer-rs/` — Rust workspace crate for the indexer
  - `src/main.rs` — entry point; Alloy provider and log subscription
  - `src/abi/ARBITRUM.json` — ABI used by the Alloy `sol!` macro for Arbitrum events (already referenced in code)

## Tech choices and rationale

- Alloy (see `Cargo.toml`: `alloy = { features = ["full"] }`): modern, fast Rust stack for Ethereum providers, types, and ABI tooling
- Tokio: async runtime
- dotenv/logging/serde: configuration and structured data handling
- Planned: `axum` for HTTP API; `sqlx` for Postgres; `brotli` for batch decompression; optional `redis` or `rocksdb` for caching/state snapshots

## Getting started

### Prerequisites

- Rust toolchain (stable)
- An Ethereum mainnet WebSocket RPC endpoint (e.g., Alchemy/Infura)

### Configuration

Create an `.env` file at `indexer-rs/.env` with:

```
# L1 Ethereum WebSocket endpoint (Alchemy, Infura, or your node)
ETHEREUM_MAINNET_WSS_URL=wss://eth-mainnet.g.alchemy.com/v2/your_key

# Arbitrum contract address to watch for batches (e.g., Inbox/Sequencer)
# Must be a 0x-prefixed address.
ARBITRUM_CONTRACT_ADDRESS=0x0000000000000000000000000000000000000000
```

These variables are read in `indexer-rs/src/main.rs`:

- `ETHEREUM_MAINNET_WSS_URL` is used to create a WebSocket provider via Alloy
- `ARBITRUM_CONTRACT_ADDRESS` is parsed as an `Address` and used to build the log `Filter`

### Run (development)

From the repository root:

```
cargo run --manifest-path indexer-rs/Cargo.toml
```

You should see a successful WebSocket connection and incoming logs printed for the subscribed contract address.

## Roadmap to full indexer

1) L1 connectivity (backfill + live)
- Use Alloy HTTP provider for backfill and WS for live heads. Filter Inbox/Sequencer events by address and topics.

2) Decode Arbitrum Nitro batch bytes
- Extract event payload, decompress with Brotli (`brotli` crate), parse Nitro envelope (per Arbitrum docs) into L2 messages/transactions.

3) Uniswap v3 calldata & events decoding
- Use Alloy ABI tooling/`abigen` to decode Uniswap v3 pool/router calls (swap, mint, burn) and their event logs. Restrict scope to these calls for MVP.

4) Persistence schema (Postgres via `sqlx`)
- Tables like `l1_batches`, `l2_txs`, `uniswap_v3_events` (swap/mint/burn normalized fields). Unique constraints for idempotency.

5) Axum API
- Endpoints: `/health`, `/blocks`, `/block/:l1_block`, `/tx/:l2_tx_hash`, `/pool/:address/events`, `/address/:addr/txs`.

6) Reorgs & reliability
- N confirmations for L1 finality; mark tentative/orphaned. Optionally snapshot state to revert. Ensure idempotent ingestion.

7) Observability & deploy
- `tracing` + JSON logs; Prometheus metrics; Docker containers for indexer and Postgres.


## Testing (TBD)

- Unit tests for Brotli decompression against sample batch payloads
- Parsing tests for Nitro message envelope to extract L2 txs
- ABI decoding tests for Uniswap v3 `swap`, `mint`, `burn` functions and emitted events
- Integration: backfill a known L1 block range and verify decoded swaps vs. Arbiscan

## References

- Alloy docs: https://alloy.rs
- Arbitrum docs (Nitro batches, Inbox/Sequencer events, parsing guidance): https://docs.arbitrum.io
- Community notes/examples on Brotli decompressing L1 batch data (search: “Arbitrum batch brotli decompress”) on blogs/StackOverflow
- Uniswap v3 ABIs: official Uniswap repositories

## License

MIT
