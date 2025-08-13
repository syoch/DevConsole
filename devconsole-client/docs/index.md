# DevConsole Client

DevConsole Clientは、DevConsoleサーバーに接続してチャンネルベースの通信を行うためのRustライブラリです。

## インストール

`Cargo.toml`に以下を追加してください：

```toml
[dependencies]
devconsole_client = { path = "../devconsole-client" }
tokio = { version = "1.47.1", features = ["macros", "rt-multi-thread", "sync"] }
```

## 基本的な使用方法

### クライアントの作成

```rust
use devconsole_client::DCClient;

let mut client = DCClient::new("ws://127.0.0.1:9001").await?;
```

WebSocketサーバーのURLを指定してクライアントを作成します。接続が成功すると、サーバーから`NodeIDNotification`を受信します。

### チャンネルの作成

```rust
let channel_id = client.open("MyChannel".to_string()).await?;
```

指定した名前でチャンネルを作成し、チャンネルIDを取得します。

### チャンネルのリッスン

```rust
use tokio::sync::mpsc;

let (tx, mut rx) = mpsc::channel(64);
client.listen(channel_id, tx).await?;

// データ受信のハンドリング
tokio::spawn(async move {
    while let Some((channel, data)) = rx.recv().await {
        println!("Received on channel {}: {}", channel, data);
    }
});
```

指定したチャンネルからのデータを受信するためのリッスンを開始します。

### データの送信

```rust
client.send(channel_id, "Hello, World!".to_string()).await?;
```

指定したチャンネルにデータを送信します。

### チャンネル一覧の取得

```rust
let channels = client.channel_list().await?;
```

現在利用可能なチャンネルの一覧を取得します。

## APIリファレンス

### DCClient

#### new(url: &str) -> Result<Self, tungstenite::Error>

指定されたURLのDevConsoleサーバーに接続します。

- **url**: WebSocketサーバーのURL（例: "ws://127.0.0.1:9001"）
- **戻り値**: DCClientインスタンスまたはWebSocketエラー
- **動作**: WebSocket接続を確立し、内部でメッセージ受信用のタスクを起動

#### open(&mut self, name: String) -> Result<ChannelID, DCClientError>

新しいチャンネルを作成します。

- **name**: チャンネル名（任意の文字列）
- **戻り値**: 作成されたチャンネルのIDまたはエラー
- **動作**: サーバーに`ChannelOpenRequest`を送信し、`ChannelOpenResponse`を待機

#### listen(&mut self, channel: ChannelID, channel_tx: mpsc::Sender<(ChannelID, String)>) -> Result<(), DCClientError>

指定されたチャンネルをリッスンし、受信したデータを指定されたチャンネルに送信します。

- **channel**: リッスンするチャンネルID
- **channel_tx**: データ受信時に通知されるmpscチャンネル
- **戻り値**: 成功時は`()`、失敗時はエラー
- **動作**:
  - 既にリッスン中の場合は警告のみで成功を返す
  - サーバーに`ChannelListenRequest`を送信
  - データハンドラーを内部で登録

#### send(&mut self, channel: ChannelID, data: String) -> Result<(), DCClientError>

指定されたチャンネルにデータを送信します。

- **channel**: 送信先チャンネルID
- **data**: 送信するデータ（任意の文字列）
- **戻り値**: 成功時は`()`、失敗時はエラー
- **動作**: サーバーに`Data`イベントを送信

#### channel_list(&mut self) -> Result<Vec<ChannelID>, DCClientError>

現在利用可能なチャンネルの一覧を取得します。

- **戻り値**: チャンネルIDのリストまたはエラー
- **動作**: サーバーに`ChannelListRequest`を送信し、`ChannelListResponse`を待機

### DCClientError

クライアント操作で発生する可能性のあるエラー：

```rust
#[derive(Debug)]
pub enum DCClientError {
    WSError(tungstenite::Error),    // WebSocket通信エラー
    ConnectionBroken,               // 接続が切断された
}
```

- `WSError`: WebSocketレベルでのエラー（ネットワークエラー、プロトコルエラーなど）
- `ConnectionBroken`: サーバーとの接続が予期せず切断された

## 内部アーキテクチャ

### コンポーネント構成

```
DCClient
├── tx: SplitSink<WebSocketStream>     # WebSocket送信ストリーム
├── dispatches: SharedDispatchers       # イベントディスパッチャー
└── listening_channels: Vec<ChannelID>  # リッスン中チャンネル

SharedDispatchers
├── events: HashMap<DispatchID, oneshot::Sender<bool>>  # イベント応答待機
├── resolve_channel: Option<oneshot::Sender<ChannelID>> # チャンネル作成応答
├── channel_list: Option<oneshot::Sender<Vec<ChannelID>>> # チャンネル一覧応答
└── data_handlers: HashMap<ChannelID, mpsc::Sender<...>> # データハンドラー
```

### メッセージ処理フロー

1. **送信**: DCClient → WebSocket送信ストリーム → サーバー
2. **受信**: サーバー → WebSocket受信ストリーム → 内部タスク → ディスパッチャー
3. **配信**: ディスパッチャー → 各種ハンドラー（応答待機、データ受信）

## 使用例

### 基本的な送受信例

```rust
use devconsole_client::DCClient;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DCClient::new("ws://127.0.0.1:9001").await?;

    // チャンネルを作成
    let channel = client.open("TestChannel".to_string()).await?;

    // リッスン開始
    let (tx, mut rx) = mpsc::channel(64);
    client.listen(channel, tx).await?;

    // データ受信の処理
    tokio::spawn(async move {
        while let Some((channel_id, data)) = rx.recv().await {
            println!("Channel {}: {}", channel_id, data);
        }
    });

    // データ送信
    client.send(channel, "Hello from client!".to_string()).await?;

    Ok(())
}
```

### 既存チャンネルの監視例

```rust
use devconsole_client::DCClient;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DCClient::new("ws://127.0.0.1:9001").await?;

    // 利用可能なチャンネルを取得
    let channels = client.channel_list().await?;

    let (tx, mut rx) = mpsc::channel(64);

    // 全チャンネルをリッスン
    for channel in channels {
        client.listen(channel, tx.clone()).await?;
    }

    // データ受信処理
    while let Some((channel_id, data)) = rx.recv().await {
        println!("Received on channel {}: {}", channel_id, data);
    }

    Ok(())
}
```

### エラーハンドリング例

```rust
use devconsole_client::{DCClient, DCClientError};

async fn robust_send(
    client: &mut DCClient,
    channel: u64,
    data: String
) -> Result<(), String> {
    match client.send(channel, data).await {
        Ok(()) => Ok(()),
        Err(DCClientError::WSError(e)) => {
            eprintln!("WebSocket error: {}", e);
            Err(format!("Send failed: {}", e))
        }
        Err(DCClientError::ConnectionBroken) => {
            eprintln!("Connection broken");
            Err("Connection lost".to_string())
        }
    }
}
```

### JSON構造化データの送受信例

```rust
use serde::{Serialize, Deserialize};
use devconsole_client::DCClient;

#[derive(Serialize, Deserialize)]
struct SensorData {
    temperature: f32,
    humidity: f32,
    timestamp: u64,
}

async fn send_sensor_data(
    client: &mut DCClient,
    channel: u64,
    data: SensorData
) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string(&data)?;
    client.send(channel, json).await?;
    Ok(())
}

async fn receive_sensor_data(data: String) -> Result<SensorData, serde_json::Error> {
    serde_json::from_str(&data)
}
```

## 非同期プログラミングモデル

### Tokio統合

このライブラリはTokioベースの非同期プログラミングモデルを使用しています：

- すべてのメソッドは`async`関数
- 内部でTokioタスクを使用してWebSocket通信を管理
- `mpsc`チャンネルを使用したデータ受信パターン

### 並行処理

```rust
use tokio::join;

// 複数の操作を並行実行
let (channel1, channel2) = join!(
    client.open("Channel1".to_string()),
    client.open("Channel2".to_string())
);
```

## 注意事項とベストプラクティス

### 制限事項

- 同じチャンネルに対して複数回`listen()`を呼び出すと警告が出力されますが、エラーにはなりません
- クライアントが切断されると、そのクライアントが作成したチャンネルは自動的に削除されます
- WebSocket接続の自動再接続機能は実装されていません
- `ChannelInfoRequest`の機能は部分的に実装されています

### パフォーマンス考慮事項

- **メッセージバッファリング**: mpscチャンネルのバッファサイズを適切に設定
- **リッスンチャンネル数**: 多数のチャンネルをリッスンする場合はメモリ使用量に注意
- **送信頻度**: 高頻度でメッセージを送信する場合はWebSocketのバックプレッシャーに注意

### エラーハンドリングのベストプラクティス

1. **接続エラー**: `new()`呼び出し時のネットワークエラーを適切にハンドリング
2. **送信エラー**: `WSError`と`ConnectionBroken`を区別して処理
3. **受信エラー**: mpscチャンネルの切断を適切に検出
4. **タイムアウト**: 長時間の応答待機にタイムアウトを設定

```rust
use tokio::time::{timeout, Duration};

// タイムアウト付きチャンネル作成
let channel = timeout(
    Duration::from_secs(5),
    client.open("MyChannel".to_string())
).await??;
```

## 使用例

### [基本的な送受信](./examples/basic.md)
チャンネルの作成、データの送受信の基本的な使い方を示します。

### [複数チャンネル監視](./examples/multi-channel.md)
複数のチャンネルを同時に監視する方法を説明します。

### [エラーハンドリング](./examples/error-handling.md)
適切なエラーハンドリングの実装方法を詳しく解説します。

## 設定とカスタマイズ

### 接続設定

```rust
// ローカル接続
let client = DCClient::new("ws://127.0.0.1:9001").await?;

// リモート接続
let client = DCClient::new("ws://remote-server:9001").await?;

// セキュア接続（WSS）
let client = DCClient::new("wss://secure-server:9001").await?;
```

### データ受信バッファ

```rust
// 大量データを扱う場合はバッファサイズを調整
let (tx, mut rx) = mpsc::channel(1024); // デフォルトは64
```

## デバッグとトラブルシューティング

### ログの有効化

```bash
RUST_LOG=debug cargo run --example your_example
```

### デバッグのヒント

1. **接続確認**: まずサーバーが起動しているか確認
2. **チャンネル状態**: `channel_list()`で現在のチャンネル状態を確認
3. **ログ出力**: 詳細なログでイベントの流れを追跡

## サポート

問題が発生した場合：

1. [基本使用例](./examples/basic.md)を確認
2. [エラーハンドリング例](./examples/error-handling.md)を参考にエラー処理を実装
3. デバッグログを有効にして問題を特定
4. サーバーログも併せて確認
