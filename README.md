# Ultra-Fast Manual Trading Execution Bot (Rust)

Prototype of an **ultra-low latency manual trading executor** designed for prediction markets such as Polymarket.

The system allows a trader to **instantly execute pre-prepared orders** by pressing a keyboard shortcut.

Orders are **prepared in advance and stored in memory**, so the execution path performs **no heavy computation** and only dispatches network requests.

Target use case: **manual high-speed trading during live events**.

---

# Key Features

• **Ultra-Low Latency Execution**  
Orders are pre-built and stored in lock-free structures.

• **Parallel Order Dispatch**  
Goal market and 1X2 market orders are sent simultaneously.

• **Multi-Match Support**  
Each match has its own execution shortcut.

• **Keyboard Trading Interface**  
Minimal terminal UI designed for fastest possible execution.

• **HFT-Style Architecture**  
Execution path performs **zero allocations and no computation**.

---

# Architecture

The system follows a 3-stage pipeline optimized for speed.
