# DevConsole AI Coding Instructions

## Architecture Overview

DevConsole is a channel-based WebSocket communication system with 5 Rust workspace members:

- **devconsole-server**: WebSocket server (port 9001) managing client connections and message routing
- **devconsole-client**: Async library for connecting to server with channel operations
- **devconsole-protocol**: Shared event definitions and data types (serde-based JSON)
- **devconsole-serial-monitor**: App that monitors USB/ACM devices and forwards data to channels
- **devconsole-data-logger**: App that listens to all channels and logs received data

## Key Design Patterns

### Protocol-First Communication

All client-server communication uses the `Event` enum in `devconsole-protocol`. Always use existing event types:

```rust
// Example: Channel creation flow
Event::ChannelOpenRequest { name } → Event::ChannelOpenResponse { channel, success }
```

### Shared Types & Arc<Mutex<T>> Pattern

- `SharedServer(Arc<Mutex<Server>>)` and `SharedClient(Arc<Mutex<Client>>)` for thread-safe state
- `ChannelID` and `NodeID` are `u64` type aliases
- Always use `.lock().await` for async access to shared state

### Async/Tokio Patterns

- All networking is tokio-based with WebSocket streams split into `(SplitSink, SplitStream)`
- Use `oneshot::channel` for request-response patterns
- Use `mpsc::channel` for data streaming (see DCClient data handlers)

## Critical Development Workflows

### Running Components

```bash
# Server must start first (port 9001)
cargo run --bin devconsole_server

# Applications that create/use channels
cargo run --bin devconsole_serial_monitor
cargo run --bin devconsole_data_logger

# Debug with detailed logs
RUST_LOG=debug cargo run --bin devconsole_server
```

### Testing WebSocket Protocol

Use wscat or similar to test server directly:

```bash
wscat -c ws://127.0.0.1:9001
# Send: {"ChannelOpenRequest": {"name": "TestChannel"}}
```

## Project-Specific Conventions

### Event Handling in Server

Server processes events in `client_handler()` with match statements. New events require:

1. Add to protocol enum
2. Add match arm in server
3. Update client library if needed

### Channel Lifecycle

- Channels are created by clients via `ChannelOpenRequest`
- Channels auto-delete when creating client disconnects
- Multiple clients can listen to same channel via `ChannelListenRequest`
- Data sent to channel broadcasts to all listeners

### Error Handling Pattern

- Server uses `Result<(), String>` for errors
- Client uses custom `DCClientError` enum
- Use `unwrap()` for "should never fail" cases, `map_err()` for propagation

## Integration Points

### DevConsole Protocol Examples

Core WebSocket communication events:

```json
// Channel creation
{"ChannelOpenRequest": {"name": "MyChannel"}}
{"ChannelOpenResponse": {"channel": 1, "success": true}}

// Channel listening
{"ChannelListenRequest": {"channel": 1}}
{"ChannelListenResponse": {"channel": 1, "success": true}}

// Data transmission
{"Data": {"channel": 1, "data": "Hello World"}}

// Channel discovery
{"ChannelListRequest": {}}
{"ChannelListResponse": {"channels": [1, 2, 3]}}

// Node ID assignment (server → client)
{"NodeIDNotification": {"node_id": 123}}
```

### Serial Device Integration

Serial monitor creates "SerialMonitor" channel with JSON events:

```json
{"Opened": {"path": "/dev/ttyACM0"}}
{"Line": {"path": "/dev/ttyACM0", "line": "data"}}
{"Closed": {"path": "/dev/ttyACM0"}}
```

### Client Library Integration

DCClient provides async API. Key pattern for data receiving:

```rust
let (tx, mut rx) = mpsc::channel(64);
client.listen(channel_id, tx).await?;
while let Some((channel, data)) = rx.recv().await {
    // Process data
}
```

## Build System Notes

- Workspace root Cargo.toml defines 5 members
- Nix flake provides dev environment with udev (for serial devices)
- No authentication/authorization implemented
- No data persistence - server restart clears all state

## Debugging Tips

- Enable debug logs with `RUST_LOG=debug`
- Server logs show connection/disconnection and event processing
- Client library logs show WebSocket message flow
- Check channel existence with `ChannelListRequest` if data not flowing

## Language and Communication

- **日本語対応**: このプロジェクトの開発者は日本語話者です。質問や説明は日本語で行ってください
- **Documentation**: 既存のドキュメントは日本語で書かれているため、新しいドキュメントも日本語で作成してください
- **Code Comments**: 複雑なロジックには日本語でコメントを追加することを推奨します
