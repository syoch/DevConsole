# DevConsole Protocol

DevConsole Protocolは、サーバーとクライアント間のWebSocket通信で使用されるイベント定義とデータ型を提供します。

## インストール

`Cargo.toml`に以下を追加してください：

```toml
[dependencies]
devconsole_protocol = { path = "../devconsole-protocol" }
serde = { version = "1.0", features = ["derive"] }
```

## データ型

### 基本型

```rust
pub type ChannelID = u64;
pub type NodeID = u64;
```

- `ChannelID`: チャンネルを一意に識別するID（64ビット符号なし整数）
- `NodeID`: 接続されたクライアント（ノード）を一意に識別するID（64ビット符号なし整数）

### エラー型

```rust
pub enum TransactionError {
    ChannelConflicted,
}
```

## イベント定義

### NodeIDNotification
サーバーからクライアントに送信される、ノードIDの通知。

```rust
NodeIDNotification {
    node_id: NodeID,
}
```

**用途**: クライアントの接続時にサーバーから送信され、そのクライアントのノードIDを通知する。

### Data
チャンネル上でのデータ送受信に使用されるイベント。

```rust
Data {
    channel: ChannelID,
    data: String,
}
```

**用途**:
- クライアントからサーバーへ：指定されたチャンネルにデータを送信
- サーバーからクライアントへ：リッスン中のチャンネルでデータを受信

### チャンネル管理

#### ChannelOpenRequest/Response
チャンネルの作成要求と応答。

```rust
ChannelOpenRequest {
    name: String,
}

ChannelOpenResponse {
    channel: ChannelID,
    success: bool,
}
```

**用途**:
- `ChannelOpenRequest`: 新しいチャンネルの作成を要求
- `ChannelOpenResponse`: チャンネル作成結果を応答（成功時はチャンネルIDを含む）

#### ChannelCloseRequest
チャンネルの閉鎖要求。

```rust
ChannelCloseRequest {
    channel: ChannelID,
}
```

**用途**: 指定されたチャンネルの閉鎖を要求

#### ChannelListenRequest/Response
チャンネルのリッスン開始要求と応答。

```rust
ChannelListenRequest {
    channel: ChannelID,
}

ChannelListenResponse {
    channel: ChannelID,
    success: bool,
}
```

**用途**:
- `ChannelListenRequest`: 指定されたチャンネルのリッスン開始を要求
- `ChannelListenResponse`: リッスン開始結果を応答

### チャンネル情報取得

#### ChannelListRequest/Response
利用可能なチャンネル一覧の取得。

```rust
ChannelListRequest

ChannelListResponse {
    channels: Vec<ChannelID>,
}
```

**用途**:
- `ChannelListRequest`: 現在利用可能なチャンネル一覧を要求
- `ChannelListResponse`: チャンネルIDのリストを応答

#### ChannelInfoRequest/Response
特定のチャンネルの詳細情報取得。

```rust
ChannelInfoRequest(ChannelID)

ChannelInfoResponse {
    channel: ChannelID,
    name: String,
    supplied_by: NodeID,
}
```

**用途**:
- `ChannelInfoRequest`: 指定されたチャンネルの詳細情報を要求
- `ChannelInfoResponse`: チャンネル名と作成者ノードIDを応答

## 通信フロー

### 基本的な接続フロー

```
1. WebSocket接続確立
2. NodeIDNotification（サーバー → クライアント）
3. チャンネル操作（作成・リッスン・データ送信）
```

### チャンネル作成フロー

```
Client                    Server
  |                         |
  |─ ChannelOpenRequest ──►|
  |   { name: "MyChannel" } |
  |                         |
  |◄── ChannelOpenResponse ─|
  |   { channel: 1,         |
  |     success: true }     |
  |                         |
```

### リッスン開始フロー

```
Client                    Server
  |                         |
  |─ ChannelListenRequest ►|
  |   { channel: 1 }        |
  |                         |
  |◄─ ChannelListenResponse─|
  |   { channel: 1,         |
  |     success: true }     |
  |                         |
```

### データ送信フロー

```
Client A                 Server                 Client B (Listener)
   |                       |                         |
   |─── Data ─────────────►|                         |
   |   { channel: 1,       |                         |
   |     data: "Hello" }   |                         |
   |                       |──── Data ──────────────►|
   |                       |   { channel: 1,         |
   |                       |     data: "Hello" }     |
   |                       |                         |
```

### チャンネル一覧取得フロー

```
Client                    Server
  |                         |
  |─ ChannelListRequest ──►|
  |                         |
  |◄── ChannelListResponse ─|
  |   { channels: [1, 2] }  |
  |                         |
```

## ライブラリ使用例

### イベントの作成と操作

```rust
use devconsole_protocol::Event;
use serde_json;

// データ送信イベントの作成
let event = Event::Data {
    channel: 1,
    data: "Hello, World!".to_string(),
};

// JSONシリアライゼーション
let json = serde_json::to_string(&event).unwrap();

// JSONデシリアライゼーション
let parsed_event: Event = serde_json::from_str(&json).unwrap();
```

### イベントの作成と送信

```rust
use devconsole_protocol::Event;

// データ送信イベント
let data_event = Event::Data {
    channel: 1,
    data: "sensor reading: 25.3°C".to_string(),
};

// チャンネル作成要求
let open_request = Event::ChannelOpenRequest {
    name: "SensorData".to_string(),
};

// チャンネルリッスン要求
let listen_request = Event::ChannelListenRequest {
    channel: 1,
};
```

### イベントのパターンマッチング

```rust
use devconsole_protocol::Event;

fn handle_event(event: Event) {
    match event {
        Event::NodeIDNotification { node_id } => {
            println!("Assigned node ID: {}", node_id);
        }
        Event::Data { channel, data } => {
            println!("Data on channel {}: {}", channel, data);
        }
        Event::ChannelOpenResponse { channel, success } => {
            if success {
                println!("Channel {} created successfully", channel);
            } else {
                println!("Failed to create channel");
            }
        }
        Event::ChannelListResponse { channels } => {
            println!("Available channels: {:?}", channels);
        }
        _ => {
            println!("Unhandled event: {:?}", event);
        }
    }
}
```

## カスタムデータ構造

DevConsoleではデータはString型で送信されるため、構造化データを送信する場合はJSONエンコーディングを使用します：

```rust
use serde::{Serialize, Deserialize};
use devconsole_protocol::Event;

#[derive(Serialize, Deserialize)]
struct SensorReading {
    temperature: f32,
    humidity: f32,
    timestamp: u64,
}

// 送信側
let reading = SensorReading {
    temperature: 25.3,
    humidity: 60.2,
    timestamp: 1234567890,
};

let data_json = serde_json::to_string(&reading).unwrap();
let event = Event::Data {
    channel: 1,
    data: data_json,
};

// 受信側
if let Event::Data { data, .. } = event {
    let reading: SensorReading = serde_json::from_str(&data).unwrap();
    println!("Temperature: {}°C", reading.temperature);
}
```

## エラーハンドリング

```rust
use devconsole_protocol::{Event, TransactionError};
use serde_json;

fn safe_parse_event(json: &str) -> Result<Event, serde_json::Error> {
    serde_json::from_str(json)
}

// 使用例
match safe_parse_event(received_json) {
    Ok(event) => handle_event(event),
    Err(e) => eprintln!("Failed to parse event: {}", e),
}
```

### エラー型

```rust
use devconsole_protocol::TransactionError;

// チャンネル競合エラー
let error = TransactionError::ChannelConflicted;
```

## イベント処理の順序

1. **接続時**:
   - WebSocket接続確立
   - `NodeIDNotification`受信

2. **チャンネル作成時**:
   - `ChannelOpenRequest`送信
   - `ChannelOpenResponse`受信

3. **データ通信時**:
   - `ChannelListenRequest`送信（リッスン開始）
   - `ChannelListenResponse`受信
   - `Data`送信/受信

4. **切断時**:
   - WebSocket接続終了
   - 作成したチャンネルの自動削除

## シリアライゼーション

- すべてのイベントはJSON形式でシリアライズされます
- WebSocketのテキストメッセージとして送信されます
- Rustの`serde`ライブラリを使用してシリアライゼーション/デシリアライゼーションを実行
- イベントの構造は`Event`列挙型として定義され、各バリアントがイベントタイプに対応

## プロトコル仕様の詳細

### JSON形式の例

```json
// NodeIDNotification
{"NodeIDNotification": {"node_id": 123}}

// Data
{"Data": {"channel": 1, "data": "Hello World"}}

// ChannelOpenRequest
{"ChannelOpenRequest": {"name": "MyChannel"}}

// ChannelOpenResponse
{"ChannelOpenResponse": {"channel": 1, "success": true}}

// ChannelListenRequest
{"ChannelListenRequest": {"channel": 1}}

// ChannelListResponse
{"ChannelListResponse": {"channels": [1, 2, 3]}}

// ChannelInfoResponse
{"ChannelInfoResponse": {
  "channel": 1,
  "name": "MyChannel",
  "supplied_by": 123
}}
```

### プロトコルエラーハンドリング

- プロトコルレベルでのエラーは`success: false`で表現
- WebSocketレベルのエラーはWebSocket仕様に従う
- 不正なJSONや未知のイベントは無視またはログ出力

## プロトコルバージョン管理

現在のプロトコルバージョンは1.0です。将来的な変更に備えて、以下の点に注意してください：

- 新しいイベント型の追加は後方互換性を維持
- 既存のイベント構造の変更は慎重に検討
- 不明なイベントは適切に処理（無視またはエラー）

## 注意事項

- すべてのイベントはJSON形式でシリアライズされます
- データフィールドは任意の文字列を格納できますが、構造化データにはJSONの使用を推奨
- チャンネルIDとノードIDは64ビット符号なし整数です
- イベントの順序は保証されますが、異なるチャンネル間での順序は保証されません
- `Debug`トレイトが実装されているため、デバッグ時の出力が可能です

## 開発時の注意

- イベント定義を変更する場合は、すべてのコンポーネント（サーバー・クライアント）で整合性を保つこと
- 新しいイベントを追加する場合は、適切なドキュメント更新も実施すること
- テスト時はJSONシリアライゼーション/デシリアライゼーションの動作確認を実施すること
