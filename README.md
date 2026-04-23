## Matching Engine

A real-time order matching engine that matches buy and sell orders using an in-memory orderbook.

---

## Running

Requires Redis on localhost:6379.

```bash
# Start Redis
docker run -d -p 6379:6379 --name redis redis:7

# Terminal 1: matcher + API on port 3000
RUST_LOG=info cargo run -- --matcher

# Terminal 2: gateway-only API on port 3001
RUST_LOG=info PORT=3001 cargo run

# Submit an order
curl -X POST localhost:3000/orders \
  -H 'content-type: application/json' \
  -d '{"side":"Buy","price":100,"qty":5}'

# Read the book
curl localhost:3000/orderbook

# Subscribe to fills
websocat ws://localhost:3000/ws
```

## Features

- Order matching engine (price-based matching)
- REST API for submitting orders and viewing the orderbook
- Real-time trade updates via WebSocket
- Support for multiple API server instances

## Implementation Steps

#### The system was built in stages:

- Implemented core order matching logic for buy and sell orders
- Added REST endpoints:
  - `POST /orders` to submit trades
  - `GET /orderbook` to view current state
- Integrated WebSocket feed for real-time trade updates
- Added Redis to support multiple API instances and coordination

## Design Decisions

### 1. How does your system handle multiple API server instances without double-matching an order?

The orderbook is inherently a **single-writer system**. Matching is a read-modify-write process where each step depends on the current state. If multiple instances performed matching concurrently, it would lead to double-matching, phantom fills, and inconsistent state.

**Solution**:-

- All API servers act as stateless gateways. They accept requests, generate a unique order ID using Redis (INCR), and push orders into a shared Redis queue.

- A single matcher instance reads from this queue and processes orders using its in-memory orderbook. This ensures only one component updates the state, avoiding conflicts.

- When trades happen, the matcher publishes fills via Redis pub/sub. All API instances listen and forward updates to their WebSocket clients.

The latest orderbook snapshot is stored in Redis so any instance can serve GET /orderbook.

### 2. What data structure did you use for the order book and why?

I used two data structures: `BTreeMap<u64, VecDeque<Order>>`, one for bids and one for asks.

- `BTreeMap` keeps price levels sorted. `first_entry()` and `last_entry()` provide efficient access to the best prices.
- `VecDeque` stores orders at each price level in FIFO order, ensuring time priority within the same price.

### 3. What breaks first if this were under real production load?

Several things, in roughly the order they'd be:

- **Matcher Throughput**:- The matcher is a **single-threaded, single-writer component**, which limits throughput. At higher loads, it becomes the system’s ceiling.
- **Redis Round-Trip Latency:** Each order involves a network round-trip — the gateway pushes (`LPUSH`) and the matcher consumes (`LPOP`). This adds ~1ms latency per order, which becomes noticeable at scale.
- **WebSocket Backpressure**:- Fill events are broadcast using a `tokio::broadcast` channel. If a client is slow It can fall behind and The channel may drop messages. This affects **delivery guarantees**, though not core matching correctness.
- **Error handling**:- The matcher uses `.unwrap()` liberally; one bad payload or Redis blip would panic the task and halt the whole system.
- **No persistence**:- If the matcher crashes mid-match, in-flight orders in the Redis list survive, but the matcher's in-memory book state is lost on restart, the snapshot in Redis would need to be used to rehydrate.

### 4. What would you build next with another 4 hours?

- **Order cancellation.** `DELETE /orders/:id`, the `HashMap<id, (side, price)>` index is half-built for this already.

- **Crash recovery.** Use Redis Streams with consumer groups + acknowledgements instead of a plain list, so the matcher can safely resume after a restart without losing or reprocessing orders.

- **Better matching fairness.** Ensure strict price-time priority by tracking exact insertion order per price level more explicitly and testing edge cases around partial fills.

- **WebSocket Backpressure:** Fill events are broadcast using a `tokio::broadcast` channel. Slow clients can fall behind, and messages may be dropped. This impacts delivery guarantees, though not matching correctness.
-
- **Basic metrics.** Track simple stats like orders processed, matches per second, and queue depth to understand system behavior under load.
