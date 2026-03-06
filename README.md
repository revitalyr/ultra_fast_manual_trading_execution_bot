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
                     │
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

# Latency Optimization Strategy

The system minimizes click-to-execution latency by reducing work performed during the execution trigger.

## Pre-Built Orders

Orders are continuously prepared in the background and stored in memory.

Execution only loads already prepared payloads.

## Zero Allocation Execution Path

The execution path avoids:

- heap allocations
- JSON serialization
- order construction
- cryptographic signing

All expensive operations are performed before execution.

## Lock-Free Data Access

Prepared orders are stored using atomic pointer swaps (ArcSwap) allowing:

- lock-free reads
- constant-time access
- no blocking during execution

## Parallel Order Dispatch

Orders for different markets are dispatched simultaneously using async tasks.

Example:

tokio::join!(
    send_order(goal_market),
    send_order(match_market)
);

## Persistent Network Connections

HTTP connections are reused to avoid TCP and TLS handshake latency.

---

# Execution Pipeline

Keypress  
↓  
Load prepared orders (atomic pointer read)  
↓  
Parallel network dispatch  

Target execution latency:

< 2 ms click-to-order submission

---

# Example Interface

Ultra Fast Manual Trading Executor

Press 1-9 to execute matches instantly.

1  Arsenal vs Chelsea  
2  Real Madrid vs Barcelona  
3  PSG vs Marseille  

Pressing a key triggers:

EXECUTE MATCH 1

Sending goal market order  
Sending match result order  

Orders dispatched in parallel.

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

# Running the Prototype

Build:

cargo build --release

Run:

cargo run

Environment variables:

POLYMARKET_API_URL=https://api.polymarket.com  
POLYMARKET_API_KEY=your_api_key

---

# Project Structure

src/

market_data/  
- orderbook.rs  
- listener.rs  

execution/  
- prepared_orders.rs  
- executor.rs  
- order_builder.rs  

trading/  
- polymarket_client.rs  

match_engine/  
- engine.rs  

ui/  
- keyboard_dashboard.rs  

main.rs

---

# Low Latency Design Principles

Pre-built Orders  
Orders are continuously prepared in the background based on market data.

Lock-Free Access  
Prepared orders are stored using atomic pointer swaps.

Zero Execution Overhead  
Execution path avoids:

- heap allocations
- complex calculations
- blocking operations

Parallel Dispatch  
Orders are sent concurrently for minimal delay.

---

# Use Case

This architecture is designed for scenarios where human decision speed must be combined with machine-level execution speed.

Examples:

- live sports prediction markets
- event-driven trading
- manual arbitrage strategies

---

# License

MIT