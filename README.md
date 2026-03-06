# Ultra Fast Manual Trading Execution Bot

A high-performance Rust trading bot designed for ultra-low latency manual execution on Polymarket. The system allows users to manually trigger trades by pressing keyboard shortcuts, with orders pre-built and pre-signed for sub-millisecond execution times.

## Features

- **Ultra-Low Latency Execution**: Pre-built and pre-signed orders stored in lock-free memory structures
- **Parallel Order Submission**: Both goal market and match result orders sent simultaneously
- **Multi-Match Support**: Handle multiple matches simultaneously with individual keyboard shortcuts
- **Keyboard Interface**: Simple terminal-based UI for instant execution
- **HFT-Style Architecture**: Zero-allocation execution path with lock-free data structures

## Architecture

The system is built with a 3-layer architecture optimized for speed:

1. **Market Data Layer**: WebSocket connections for real-time orderbook updates
2. **Order Preparation Layer**: Continuously rebuilds executable orders based on market data
3. **Execution Fast Path**: Zero-computation path that only dispatches pre-prepared orders

## Quick Start

### Prerequisites

- Rust 1.70+ 
- Tokio runtime
- Polymarket API credentials

### Installation

```bash
git clone <repository>
cd ultra_fast_manual_trading_execution_bot
cargo build --release
```

### Configuration

Set environment variables:

```bash
export POLYMARKET_API_URL="https://api.polymarket.com"
export POLYMARKET_API_KEY="your_api_key_here"
```

### Running

```bash
# Development mode
cargo run

# Production mode (optimized)
cargo run --release
```

## Usage

### Keyboard Interface

When the application starts, you'll see a dashboard with active matches:

```
╔════════════════════════════════════════════════════════════════╗
║           Ultra Fast Manual Trading Execution Bot             ║
║                     Press 1-9 to Execute                      ║
║                          Press Q to Quit                       ║
╚════════════════════════════════════════════════════════════════╝

Active Matches:
┌─────┬────────────────────────────────────────────────────────┐
│ Key │ Match Name                                           │
├─────┼────────────────────────────────────────────────────────┤
│  1  │ Arsenal vs Chelsea                                    │
│  2  │ Real Madrid vs Barcelona                             │
│  3  │ PSG vs Marseille                                     │
└─────┴────────────────────────────────────────────────────────┘

Press a number key to execute the corresponding match instantly!
Execution will send both orders simultaneously for ultra-low latency.
```

### Execution Logic

When you press a number key:

1. **Goal Market Order**: Market order to buy all available liquidity
2. **Match Result Order**: Limit order up to predefined price limit

Both orders are sent **in parallel** for minimal latency.

## Performance

The execution path is optimized for sub-millisecond performance:

- **Order Preparation**: Done continuously in background
- **Memory Layout**: Lock-free data structures (ArcSwap, DashMap)
- **Network**: Persistent HTTP connections with connection pooling
- **Execution**: Zero computation on trigger, only network dispatch

Expected latency: **< 2ms** from keypress to order submission.

## Configuration

### Match Configuration

Matches are configured in `src/main.rs`:

```rust
MatchConfig::new(
    "match_1".to_string(),                    // Match ID
    "Arsenal vs Chelsea".to_string(),        // Display name
    "goal_market_arsenal_chelsea".to_string(), // Goal market ID
    "match_result_arsenal_chelsea".to_string(), // Match result market ID
    0.95,                                   // Max price limit
    Some('1'),                               // Keyboard shortcut
)
```

### Price Limits

Set maximum price limits to prevent overpaying:

```rust
// Maximum price for match result orders
max_price_limit: 0.95  // 95 cents max
```

## Development

### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Check for issues
cargo check
cargo clippy
```

### Project Structure

```
src/
├── main.rs                 # Application entry point
├── market_data/           # Market data handling
│   ├── orderbook.rs       # OrderBook structures
│   └── listener.rs        # WebSocket market data listener
├── execution/            # Order execution engine
│   ├── prepared_orders.rs # Pre-built order structures
│   ├── executor.rs       # Ultra-fast execution engine
│   └── order_builder.rs # Order preparation logic
├── trading/              # Polymarket API client
│   └── polymarket_client.rs
├── match_engine/         # Match management
│   └── engine.rs
└── ui/                  # User interface
    └── keyboard_dashboard.rs
```

### Key Technologies

- **Tokio**: Async runtime for high-performance I/O
- **ArcSwap**: Lock-free atomic pointer swapping
- **DashMap**: Concurrent hashmap for shared state
- **Reqwest**: HTTP client with connection pooling
- **Crossterm**: Terminal UI for keyboard interface
- **Serde**: Serialization for API communication

## Deployment

### Production Deployment

1. **Build optimized binary**:
   ```bash
   cargo build --release
   ```

2. **Server setup**:
   - Linux server recommended for best performance
   - Dedicated server for maximum speed
   - Low-latency network connection

3. **Environment configuration**:
   ```bash
   export RUST_LOG=info
   export POLYMARKET_API_URL="https://api.polymarket.com"
   export POLYMARKET_API_KEY="production_key"
   ```

4. **Run**:
   ```bash
   ./target/release/ultra_fast_manual_trading_execution_bot
   ```

### Monitoring

Monitor application logs:

```bash
# Enable detailed logging
RUST_LOG=debug ./target/release/ultra_fast_manual_trading_execution_bot

# Log to file
RUST_LOG=info ./target/release/ultra_fast_manual_trading_execution_bot 2>&1 | tee trading.log
```

## API Integration

### Polymarket Client

The `PolymarketClient` handles all API interactions:

```rust
let client = PolymarketClient::new(
    "https://api.polymarket.com".to_string(),
    Some(api_key)
);

// Submit orders
client.submit_order(&prepared_order).await?;

// Get market data
client.get_orderbook(market_id).await?;
```

### WebSocket Integration

Real-time market data via WebSocket:

```rust
let listener = MarketDataListener::new(ws_url, update_tx);
listener.start().await?;
```

## Testing

### Unit Tests

```bash
cargo test
```

### Integration Tests

```bash
cargo test --test integration_tests
```

### Performance Benchmarks

```bash
cargo bench
```

## Troubleshooting

### Common Issues

1. **High Latency**:
   - Check network connection quality
   - Verify server proximity to Polymarket
   - Monitor system load

2. **Order Failures**:
   - Verify API key validity
   - Check account balance
   - Review market availability

3. **Connection Issues**:
   - Check firewall settings
   - Verify WebSocket endpoint accessibility
   - Monitor network stability

### Debug Mode

Enable debug logging:

```bash
RUST_LOG=debug cargo run
```

## Contributing

1. Fork the repository
2. Create feature branch
3. Make changes
4. Add tests
5. Submit pull request

## License

This project is licensed under the MIT License.

## Performance Benchmarks

Expected execution times (measured on dedicated server):

- **KeyPress → Execution Start**: < 0.5ms
- **Order Preparation**: 0ms (pre-built)
- **Network Dispatch**: < 1ms
- **Total Latency**: < 2ms

## Security

- API keys stored in environment variables only
- No sensitive data in logs
- TLS encryption for all network communications
- Order signing validation (when enabled)

## Support

For issues and questions:
- Create GitHub issue
- Review logs for error details
- Check API status page
