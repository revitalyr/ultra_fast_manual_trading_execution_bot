# Ultra-Fast Manual Trading Execution Bot (Rust)

Prototype of an ultra-low latency manual trading executor designed for prediction markets such as Polymarket.

The system allows a trader to instantly execute pre-prepared orders by pressing a keyboard shortcut.

Orders are prepared in advance and stored in memory so the execution path performs no heavy computation and only dispatches network requests.

Target use case: manual high-speed trading during live events.

---

# Key Features

### Ultra-Low Latency Execution

Orders are pre-built and stored in lock-free structures.

### Parallel Order Dispatch

Goal market and match result orders are sent simultaneously.

### Multi-Match Support

Each match has its own keyboard shortcut.

### Keyboard Trading Interface

Minimal terminal UI designed for the fastest possible execution.

### HFT-Style Architecture

Execution path performs zero allocations and no computation.

---

# Architecture Diagram

The bot follows a 3-layer, low-latency pipeline optimized for sub-millisecond execution.

             ┌────────────────────┐
             │  Market Data Layer │
             │  (WebSocket feeds) │
             └─────────┬──────────┘
                       │
                       ▼
             ┌────────────────────┐
             │ Order Preparation  │
             │ (Pre-built Orders) │
             └─────────┬──────────┘
                       │
                       ▼
             ┌──────────────────────┐
             │ Execution Fast Path │
             │ (Zero-allocation    │
             │   dispatch)         │
             └───────┬─────────────┘
       ┌─────────────┴─────────────┐
       ▼                           ▼
      Goal Market Order Match Result Order
       │                           │
       └─────────────┬─────────────┘
                     ▼
                Polymarket API


**Legend / Notes:**

- **Market Data Layer**: Receives live orderbook updates via WebSocket. Lock-free data structures ensure zero contention.
- **Order Preparation**: Orders are continuously rebuilt and pre-signed in memory. No computation occurs on trigger.
- **Execution Fast Path**: Keypress triggers parallel network dispatch of goal + match orders. Latency < 2 ms.
- **Parallel Order Dispatch**: tokio::join! used for simultaneous order submission.
- **Lock-Free / Zero-Allocation**: ArcSwap + DashMap store prepared orders for instantaneous access.

---

# Quick Start

```bash
# Build
cargo build --release

# Copy and edit configuration
cp config.example.toml config.toml

# Set API credentials
export POLYMARKET_API_URL=https://clob.polymarket.com
export POLYMARKET_API_KEY=your_api_key

# Run
cargo run --release
```

---

# Configuration

## Environment Variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `POLYMARKET_API_URL` | No | `https://api.polymarket.com` | Base URL for the Polymarket CLOB API |
| `POLYMARKET_API_KEY` | No | — | API key sent as `Authorization: Bearer <key>` header |
| `CONFIG_PATH` | No | `config.toml` | Path to the match configuration file |

## config.toml Format

The configuration file defines which matches the bot monitors and what keyboard shortcuts trigger their execution.

```toml
[[matches]]
id = "match_1"
name = "Arsenal vs Chelsea"
goal_market_id = "123456"
match_market_id = "789012"
max_price_limit = 0.95
keyboard_shortcut = '1'

[[matches]]
id = "match_2"
name = "Real Madrid vs Barcelona"
goal_market_id = "345678"
match_market_id = "901234"
max_price_limit = 0.95
keyboard_shortcut = '2'
```

### Field Reference

| Field | Type | Description |
|---|---|---|
| `id` | string | Unique match identifier (used internally) |
| `name` | string | Human-readable name shown in the dashboard |
| `goal_market_id` | string | Polymarket market ID for the "goal" (e.g., first goal scorer) market |
| `match_market_id` | string | Polymarket market ID for the match result (win/draw) market |
| `max_price_limit` | float | Maximum acceptable price. Orders are rejected if the best available price exceeds this threshold |
| `keyboard_shortcut` | char (optional) | Single key that triggers execution for this match. Omit if no shortcut is needed |

---

# API Endpoints

The bot communicates with the Polymarket CLOB API (or any compatible endpoint). All requests include the `Authorization: Bearer <key>` header if an API key is configured.

## POST /api/v1/orders

Used by `submit_order()` / `submit_prepared_order()`.

**Request body:**

```json
{
    "marketId": "string",
    "type": "market" | "limit",
    "side": "buy" | "sell",
    "size": 0.0,
    "price": null | 0.0,
    "timestamp": 1700000000000
}
```

**Headers:**

| Header | Value |
|---|---|
| `Content-Type` | `application/json` |
| `Authorization` | `Bearer <api_key>` (if configured) |

---

## POST /api/v1/orders/submit

Used by `submit_prepared_order_internal()` — for orders that include a pre-computed signature.

**Request body:**

Same JSON payload as `/api/v1/orders`.

**Headers:**

| Header | Value |
|---|---|
| `Content-Type` | `application/json` |
| `X-Signature` | `<hex-encoded signature>` (if present) |
| `Authorization` | `Bearer <api_key>` (if configured) |

---

## GET /api/v1/markets

Used by `get_markets()`. Returns a list of available markets.

**Headers:** `Authorization: Bearer <api_key>` (if configured)

---

## GET /api/v1/markets/{marketId}/orderbook

Used by `get_orderbook()`. Returns the current order book for a given market.

**Headers:** `Authorization: Bearer <api_key>` (if configured)

**Path parameters:**

| Parameter | Description |
|---|---|
| `marketId` | Polymarket market ID |

---

## GET /api/v1/account/balance

Used by `get_balance()`. Returns the account balance.

**Headers:** `Authorization: Bearer <api_key>` (if configured)

---

## POST /api/v1/orders/{orderId}/cancel

Used by `cancel_order()`. Cancels a previously placed order.

**Headers:** `Authorization: Bearer <api_key>` (if configured)

**Path parameters:**

| Parameter | Description |
|---|---|
| `orderId` | ID of the order to cancel |

---

# Project Structure

```
src/
  market_data/
    orderbook.rs        — BTreeMap-based order book with O(log n) updates
  execution/
    prepared_orders.rs  — Order data structures, payload builder with timestamp validation
    executor.rs         — Execution engine: bounded channels, parallel dispatch, HTTP integration
    order_builder.rs    — Builds prepared orders from order book state
  trading/
    polymarket_client.rs — HTTP client wrapping all Polymarket API endpoints
  match_engine/
    engine.rs           — Match lifecycle management, market data handler
  ui/
    keyboard_dashboard.rs — Terminal UI with per-match keyboard shortcuts
  config.rs             — TOML configuration deserialization
  traits.rs             — TradingClient, ExecutionEngine, MatchManagerHandle traits
  main.rs               — Binary entry point
  lib.rs                — Library root with re-exports
config.example.toml     — Example configuration file
```

---

# Technology Stack

Rust ecosystem optimized for low latency.

Tokio — async runtime
Reqwest — HTTP client with connection pooling
ArcSwap — lock-free atomic pointer swaps
DashMap — concurrent state management
Crossterm — terminal keyboard interface
Serde — API serialization

---

# License

MIT
