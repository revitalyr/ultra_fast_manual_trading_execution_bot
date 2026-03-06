# Architecture

## Overview

The Ultra Fast Manual Trading Execution Bot is built with a 3-layer architecture optimized for sub-millisecond execution latency. The system uses HFT (High-Frequency Trading) principles with lock-free data structures and zero-allocation execution paths.

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Trading Bot Architecture                     │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │   Market Data   │  │ Order Preparation│  │  Execution Fast │ │
│  │      Layer      │  │      Layer      │  │      Path       │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Layer 1: Market Data Layer

### Purpose
Real-time data ingestion and processing from Polymarket WebSocket feeds.

### Components

#### MarketDataListener
- **WebSocket connections** to Polymarket real-time feeds
- **Message parsing** and validation
- **Data distribution** to other layers via channels
- **Connection management** with automatic reconnection

```rust
pub struct MarketDataListener {
    websocket_url: String,
    update_tx: mpsc::UnboundedSender<MarketUpdate>,
}
```

#### OrderBook
- **Lock-free orderbook** representation
- **Bid/ask price aggregation**
- **Volume tracking** and price levels
- **Atomic updates** using ArcSwap

```rust
pub struct OrderBook {
    market_id: String,
    bids: Vec<(f64, f64)>,  // (price, volume)
    asks: Vec<(f64, f64)>,  // (price, volume)
    updated_at: u64,
}
```

#### MarketUpdate
- **Unified update format** for different data types
- **Type-safe enum** for update variants
- **Timestamp tracking** for latency measurement

```rust
pub enum MarketUpdateType {
    OrderBookUpdate(OrderBook),
    TradeUpdate(Trade),
    PriceUpdate(PriceUpdate),
}
```

### Performance Optimizations
- **Zero-copy message parsing** where possible
- **Lock-free data structures** for concurrent access
- **Batch processing** of market updates
- **Connection pooling** for WebSocket streams

## Layer 2: Order Preparation Layer

### Purpose
Continuous preparation and optimization of executable orders based on market data.

### Components

#### OrderPreBuilder
- **Real-time order building** based on market conditions
- **Price limit enforcement** and risk management
- **Order size calculation** and optimization
- **Pre-signing** for instant execution

```rust
pub struct OrderPreBuilder {
    max_price_limit: f64,
    order_size_calculator: OrderSizeCalculator,
}
```

#### PreparedOrder
- **Pre-built order structures** ready for instant execution
- **Optimized memory layout** for cache efficiency
- **Pre-computed payloads** and signatures
- **Zero-allocation access** patterns

```rust
pub struct PreparedOrder {
    id: uuid::Uuid,
    market_id: String,
    order_type: OrderType,
    side: OrderSide,
    size: f64,
    price: Option<f64>,
    payload: bytes::Bytes,
    signature: Option<bytes::Bytes>,
    created_at: u64,
}
```

#### PreparedOrders
- **Dual order container** for goal + match result orders
- **Atomic swapping** using ArcSwap for lock-free updates
- **Version tracking** for consistency
- **Memory-efficient storage**

```rust
pub struct PreparedOrders {
    match_id: String,
    goal_order: PreparedOrder,
    match_order: PreparedOrder,
    updated_at: u64,
}
```

### Performance Optimizations
- **Continuous background preparation**
- **Lock-free atomic swapping** (ArcSwap)
- **Pre-computed signatures** and payloads
- **Cache-friendly memory layout**
- **Zero-allocation execution path**

## Layer 3: Execution Fast Path

### Purpose
Ultra-fast order dispatch with minimal computational overhead.

### Components

#### ExecutionEngine
- **High-performance request processing**
- **Parallel order submission**
- **Connection pooling** and HTTP optimization
- **Latency measurement** and monitoring

```rust
pub struct ExecutionEngine {
    client: Arc<PolymarketClient>,
    execution_rx: mpsc::UnboundedReceiver<ExecutionRequest>,
    prepared_orders_cache: Arc<DashMap<String, Arc<PreparedOrders>>>,
}
```

#### UltraFastExecutor
- **Zero-allocation execution logic**
- **Parallel HTTP requests** using Tokio
- **Connection reuse** and pooling
- **Sub-millisecond latency tracking**

```rust
pub struct UltraFastExecutor {
    trading_client: Arc<PolymarketClient>,
}
```

#### ExecutionRequest
- **Lightweight request structure**
- **Reference to pre-built orders**
- **Minimal memory footprint**
- **Fast serialization**

```rust
pub struct ExecutionRequest {
    match_id: String,
    prepared_orders: Arc<PreparedOrders>,
}
```

### Performance Optimizations
- **Zero computation on trigger**
- **Parallel HTTP request dispatch**
- **Connection pooling and reuse**
- **Pre-allocated buffers**
- **Lock-free request processing**

## Match Engine Integration

### MatchEngine
- **Multi-match coordination**
- **Keyboard event handling**
- **State management** for active matches
- **Performance monitoring**

```rust
pub struct MatchEngine {
    config: MatchConfig,
    prepared_orders: Arc<ArcSwap<PreparedOrders>>,
    goal_orderbook: Arc<ArcSwap<OrderBook>>,
    match_orderbook: Arc<ArcSwap<OrderBook>>,
    order_pre_builder: OrderPreBuilder,
    execution_engine: Arc<ExecutionEngine>,
    market_update_rx: mpsc::UnboundedReceiver<MarketUpdate>,
}
```

### MatchManager
- **Multiple match handling**
- **Keyboard shortcut mapping**
- **Configuration management**
- **Performance aggregation**

```rust
pub struct MatchManager {
    matches: Arc<DashMap<String, MatchConfig>>,
    execution_engine: Arc<ExecutionEngine>,
}
```

## Data Flow

### Normal Operation Flow

```
1. Market Data Ingestion
   WebSocket → MarketDataListener → MarketUpdate

2. Order Preparation
   MarketUpdate → OrderPreBuilder → PreparedOrders → ArcSwap

3. User Trigger
   Keyboard Input → MatchEngine → ExecutionRequest

4. Order Execution
   ExecutionRequest → ExecutionEngine → Parallel HTTP Requests
```

### Performance Critical Path

```
KeyPress (0ms) → MatchEngine (<0.1ms) → 
ExecutionRequest (<0.1ms) → ExecutionEngine (<0.5ms) → 
Network Dispatch (<1ms) → Total (<2ms)
```

## Memory Architecture

### Lock-Free Data Structures

#### ArcSwap Usage
- **PreparedOrders updates**: Atomic swapping without locks
- **OrderBook updates**: Consistent snapshots for order building
- **Zero contention**: Multiple readers, single writer pattern

#### DashMap Usage
- **Match configuration storage**: Concurrent access to match configs
- **Order caching**: Fast lookup by match ID
- **Performance metrics**: Thread-safe counters and gauges

### Memory Layout Optimization

#### Struct Ordering
- **Hot fields first** (frequently accessed)
- **Cache line alignment** for critical structures
- **Padding to avoid false sharing**
- **Sequential memory access patterns**

#### Allocation Strategy
- **Arena allocation** for temporary objects
- **Object pooling** for frequently created/destroyed items
- **Pre-allocation** of buffers and vectors
- **Stack allocation** where possible

## Concurrency Model

### Actor Pattern
- **MarketDataListener**: Data ingestion actor
- **OrderPreBuilder**: Order preparation actor  
- **ExecutionEngine**: Order execution actor
- **MatchEngine**: Coordination actor

### Channel Communication
- **Unbounded channels** for market data (high throughput)
- **Bounded channels** for execution requests (backpressure)
- **One-shot channels** for responses
- **Watch channels** for configuration updates

### Thread Pool Configuration
- **Tokio multi-thread scheduler**
- **Dedicated I/O threads** for network operations
- **CPU-bound threads** for order preparation
- **Blocking thread pool** for file I/O

## Performance Characteristics

### Latency Targets
- **KeyPress → Execution Start**: < 0.5ms
- **Order Preparation**: 0ms (pre-built)
- **Network Dispatch**: < 1ms
- **Total Latency**: < 2ms

### Throughput Targets
- **Market Data Updates**: 10,000+ updates/second
- **Order Executions**: 1,000+ orders/second
- **Keyboard Input**: 100+ triggers/second

### Memory Usage
- **Base Memory**: ~50MB (excluding market data)
- **Market Data**: ~10MB per active match
- **Order Cache**: ~1MB per 1000 prepared orders
- **Total**: < 100MB for typical usage

## Security Architecture

### API Security
- **Environment variable storage** for API keys
- **TLS encryption** for all network communications
- **Request signing** for order validation
- **Rate limiting** and throttling

### Data Protection
- **No sensitive data in logs**
- **Memory sanitization** for sensitive fields
- **Secure random generation** for order IDs
- **Input validation** and sanitization

## Monitoring and Observability

### Metrics Collection
- **Latency histograms** for execution times
- **Throughput counters** for orders and updates
- **Error rates** and failure tracking
- **Resource usage** (memory, CPU, network)

### Logging Strategy
- **Structured logging** with tracing
- **Performance critical paths** with minimal overhead
- **Error logging** with full context
- **Audit logging** for trading activities

### Health Checks
- **Connectivity checks** for WebSocket and HTTP
- **Performance benchmarks** running continuously
- **Memory leak detection** and monitoring
- **Circuit breaker** pattern for external dependencies

## Deployment Architecture

### Single Instance Deployment
- **All components in single process**
- **Tokio runtime** for async operations
- **Signal handling** for graceful shutdown
- **Configuration via environment variables**

### Scaling Considerations
- **Horizontal scaling** via multiple instances
- **Load balancing** for WebSocket connections
- **Shared state** via external storage (Redis)
- **Distributed locking** for coordinated operations

## Technology Rationale

### Rust Language Choice
- **Zero-cost abstractions** for performance
- **Memory safety** without garbage collection
- **Fearless concurrency** with ownership model
- **Rich ecosystem** for async programming

### Key Dependencies
- **Tokio**: Industry-standard async runtime
- **ArcSwap**: Lock-free atomic operations
- **DashMap**: High-performance concurrent hashmap
- **Reqwest**: Optimized HTTP client
- **Crossterm**: Terminal UI capabilities

### Trade-offs
- **Performance vs. Complexity**: Optimized for speed
- **Memory vs. CPU**: Memory-heavy for speed
- **Single-threaded vs. Multi-threaded**: Async multi-threaded
- **Synchronous vs. Asynchronous**: Fully async

## Future Architecture Evolution

### Potential Enhancements
- **FPGA integration** for ultra-low latency
- **Kernel bypass networking** (DPDK)
- **Shared memory IPC** for inter-process communication
- **Machine learning** for order optimization

### Extensibility Points
- **Pluggable market data sources**
- **Custom order types** and strategies
- **Multiple exchange support**
- **Advanced risk management**

This architecture provides a solid foundation for ultra-low latency trading while maintaining flexibility for future enhancements and scalability requirements.
