# Solana Arbitrage Bot

**Solana arbitrage bot** — a Rust bot that discovers and executes profitable DEX arbitrage on Solana via the [Jupiter](https://jupiter.ag) aggregator. RPC-only execution. Supports **continuous quote polling** and optional **Yellowstone gRPC** big-trade monitoring.

*Search: solana arbitrage bot · Solana arbitrage · Jupiter arbitrage bot · DEX arbitrage Rust · Solana trading bot · Yellowstone gRPC · Jupiter API.*

**Contact:** [@hodlwarden](https://t.me/hodlwarden)


### How it works

1. **Discovery** — **Polling:** sweeps a notional range in a grid across all configured base tokens concurrently, requests Jupiter quotes, keeps opportunities above min profit after fees. **Big-trades:** optional Yellowstone gRPC subscription triggers quote simulation on large flows.
2. **Execution** — Builds swap instructions via Jupiter API, advances nonce, submits via RPC with requested compute and priority fee.

**Workflow:**

![Architecture: Config → Discovery → Jupiter Quotes → Fee check → Execution](images/architecture-diagram.png)

**Profit calculation** (execute only when net profit ≥ min profit):

![Profit calculation: notional grid, gross profit, tx cost, net profit, execute if ≥ min_profit](images/profit-calculation-flow.png)

---

## Test Results

**$0.006 Profit** - 
[$77 -> $0.006 Profit](https://solscan.io/tx/4ASCHbwF2q3ZeeKJgcUx93mtTwHHYwu29bmerU3KJPmGupMziqFvnQScuam8Yx4e458TSRwd9QhxC1HSiHT6EZLc#balance_change)

**$0.011 Profit** - 
[$77 -> $0.011 Profit](https://solscan.io/tx/4uQ4sANAv6oGoBeqE28T7CNQ1fDMX7EsduA87yhhBwpVXGyspVwHokkGa9oC11UEY7Kw6DK5sdWngHgDC7hz9GAS#balance_change)

---

## Features

- **Dual discovery modes**
  - **Continuous polling** — Queries Jupiter quotes across configurable amount ranges and all base tokens **concurrently** (FuturesUnordered). Adaptive interval halves on opportunity found (min 100 ms) and gradually restores when the market is quiet.
  - **Big-trades monitor** — Subscribes to Yellowstone gRPC for large on-chain flows and reacts with quote simulation.
- **8 liquid intermediates per base token** — Each base token is tested against USDC, USDT, WSOL, mSOL, JitoSOL, JUP, ETH (Wormhole), and WBTC to maximize path coverage per round.
- **Circuit breaker** — Automatically pauses trade submission for 30 seconds after 5 consecutive failures; discovery continues while paused.
- **P&L tracker** — Tracks cumulative SOL profit, win rate, and trade counts. Stats are logged every 50 poll rounds.
- **Rate-limited Jupiter calls** — A semaphore caps concurrent Jupiter API requests at 8 to prevent rate-limiting.
- **RPC-only submission** — Transactions are submitted via your configured `submit_endpoint` using standard Solana SDK. No third-party relay dependencies.
- **Multi-token support** — Configure base tokens (e.g. USDC, SOL) with notional ranges, grid steps, and min-profit thresholds.
- **Transaction cost awareness** — Estimates fee (compute, priority, tip) and SOL price (refreshed every 5 min) to filter only profitable trades.
- **Nonce-based submission** — Uses a durable nonce account for reliable transaction lifecycle.

---

## Prerequisites

- **Rust** (stable, e.g. 1.70+): [rustup](https://rustup.rs)
- **Solana RPC** — A node or provider (e.g. Helius, QuickNode, Triton) with `submitTransaction` support.
- **Wallet** — Keypair file for the bot and a funded **nonce account**.
- **Jupiter API** — Either the public Jupiter API or a self-hosted proxy; configurable in `Config.toml`.
- **Yellowstone gRPC** (optional) — Only if you enable big-trades monitoring; requires endpoint and auth token.

---

## Quick Start

1. **Clone and build**

   ```bash
   git clone https://github.com/hodlwarden/solana-arbitrage-bot.git solana-arbitrage-bot && cd solana-arbitrage-bot
   cargo build --release
   ```

2. **Create a nonce account** (one-time setup)

   Use the Solana CLI to create and fund a durable nonce account:

   ```bash
   # Generate a nonce authority keypair (or reuse your bot wallet)
   solana-keygen new -o nonce-authority.json

   # Create the nonce account (fund it with enough SOL for rent, ~0.002 SOL)
   solana create-nonce-account nonce-account.json 0.01 --nonce-authority nonce-authority.json

   # Get the nonce account public key and paste it into Config.toml
   solana address -k nonce-account.json
   ```

   Set `nonce_account_pubkey` in your config to the address printed above.

3. **Configure**

   Copy `Config.example.toml` to `Config.toml` (or `settings.toml`) and fill in your values. **Do not commit secrets.** The app loads `settings.toml` first, then falls back to `Config.toml`. Set at minimum:

   - `signer_keypair_path`, `rpc_endpoint`, `submit_endpoint`
   - `dex_api.endpoint` (Jupiter API or proxy)
   - `strategy.nonce_account_pubkey` and `strategy.instruments`
   - `[fees]` block

4. **Run**

   ```bash
   cargo run --release
   # Or after build: ./target/release/jupiter_arbitrage_bot_offchain
   ```

   Set `RUST_LOG=info` (or `debug`) to control log level.

---

## Configuration

Configuration is TOML-based. See `Config.example.toml` for the full reference.

| Section       | Purpose |
|---------------|---------|
| `[connection]` | `signer_keypair_path`, `rpc_endpoint`, `submit_endpoint`; optional `geyser_endpoint`, `geyser_auth_token` for Yellowstone. |
| `[dex_api]`   | Jupiter API `endpoint` and optional `auth_token`. |
| `[strategy]`  | `instruments` (base tokens with mint, notional range, grid steps, min profit), `nonce_account_pubkey`, `default_quote_mint`, `polling_enabled` / `poll_interval_ms`, `geyser_watch_enabled`, `execution_enabled`. |
| `[fees]`      | `compute_unit_limit`, `priority_fee_lamports`, `relay_tip_sol`; optional `third_party_fee_profit_pct` (e.g. `0.5` = 50% of gross profit in SOL); optional `sol_price_usd` fallback. |

### Third-party fee (fixed vs profit-based)

Transaction cost includes a **base network fee** plus an optional **tip**. You can set the tip in two ways:

- **Fixed** — A constant amount in SOL per trade. Use `relay_tip_sol`.
- **Profit-based** — A fraction of the trade's **gross profit in SOL**. Use `third_party_fee_profit_pct` (0.0–1.0). When set, the tip is computed as **gross profit (in SOL) × this value**.

**Example (profit-based):**  
If gross profit is **0.1 SOL** and you set `third_party_fee_profit_pct = 0.5`, the tip is **0.05 SOL**. Net profit (after base fee and this tip) is then used to decide if the trade meets `min_profit` and is submitted.

**Config examples:**

```toml
# Fixed tip: 0.00001 SOL per trade
[fees]
relay_tip_sol = 0.00001
```

```toml
# Profit-based: 50% of gross profit in SOL as tip
[fees]
relay_tip_sol = 0.00001   # fallback when profit-based is 0
third_party_fee_profit_pct = 0.5
```

If `third_party_fee_profit_pct` is set and in range (0, 1], it overrides `relay_tip_sol` for that trade; otherwise `relay_tip_sol` is used.

---

## Project Layout

| Path        | Description |
|------------|-------------|
| `src/app/` | Configuration and runtime settings (node, swap API, strategy, fees). |
| `src/chain/` | Chain data and constants (program maps, token info, fee constants). |
| `src/engine/` | Arbitrage engine: Jupiter integration, discovery (polling + big-trades), execution, runtime (nonce, blockhash, SOL price, fee cost, circuit breaker, P&L stats). |
