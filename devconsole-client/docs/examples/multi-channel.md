# 複数チャンネル監視

この例では、複数のチャンネルを同時に監視する方法を示します。

## コード例

```rust
use devconsole_client::DCClient;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // サーバーに接続
    let mut client = DCClient::new("ws://127.0.0.1:9001").await?;
    println!("Connected to DevConsole server");

    // 複数のチャンネルを作成
    let mut channels = HashMap::new();

    for i in 1..=3 {
        let channel_name = format!("Channel{}", i);
        let channel_id = client.open(channel_name.clone()).await?;
        channels.insert(channel_name.clone(), channel_id);
        println!("Created {}: {}", channel_name, channel_id);
    }

    // 統合データ受信チャンネル
    let (tx, mut rx) = mpsc::channel(256);

    // 全チャンネルのリッスンを開始
    for (name, &channel_id) in &channels {
        client.listen(channel_id, tx.clone()).await?;
        println!("Started listening to {}: {}", name, channel_id);
    }

    // データ受信処理
    tokio::spawn(async move {
        while let Some((channel_id, data)) = rx.recv().await {
            // チャンネルIDから名前を逆引き
            let channel_name = channels.iter()
                .find(|(_, &id)| id == channel_id)
                .map(|(name, _)| name.as_str())
                .unwrap_or("Unknown");

            println!("[{}] Received: {}", channel_name, data);
        }
    });

    // 各チャンネルにデータを送信
    for round in 1..=3 {
        println!("\n--- Round {} ---", round);

        for (name, &channel_id) in &channels {
            let message = format!("Message from {} (round {})", name, round);
            client.send(channel_id, message).await?;
            sleep(Duration::from_millis(500)).await;
        }

        sleep(Duration::from_secs(1)).await;
    }

    // 既存のチャンネルも監視
    println!("\n--- Checking for existing channels ---");
    let existing_channels = client.channel_list().await?;

    for channel_id in existing_channels {
        if !channels.values().any(|&id| id == channel_id) {
            println!("Found existing channel: {}", channel_id);
            client.listen(channel_id, tx.clone()).await?;
        }
    }

    // しばらく待機して他のクライアントからのデータを監視
    println!("Monitoring for 10 seconds...");
    sleep(Duration::from_secs(10)).await;

    println!("Demo completed");
    Ok(())
}
```

## 実行方法

1. DevConsoleサーバーを起動：
```bash
cargo run --bin devconsole_server
```

2. この例を実行：
```bash
cargo run --example multi_channel
```

3. （オプション）別のターミナルでSerial Monitorを起動して追加データを確認：
```bash
cargo run --bin devconsole_serial_monitor
```

## 期待される出力

```
Connected to DevConsole server
Created Channel1: 1
Created Channel2: 2
Created Channel3: 3
Started listening to Channel1: 1
Started listening to Channel2: 2
Started listening to Channel3: 3

--- Round 1 ---
[Channel1] Received: Message from Channel1 (round 1)
[Channel2] Received: Message from Channel2 (round 1)
[Channel3] Received: Message from Channel3 (round 1)

--- Round 2 ---
[Channel1] Received: Message from Channel1 (round 2)
[Channel2] Received: Message from Channel2 (round 2)
[Channel3] Received: Message from Channel3 (round 2)

--- Round 3 ---
[Channel1] Received: Message from Channel1 (round 3)
[Channel2] Received: Message from Channel2 (round 3)
[Channel3] Received: Message from Channel3 (round 3)

--- Checking for existing channels ---
Monitoring for 10 seconds...
Demo completed
```

## 高度な例：チャンネル別処理

```rust
use devconsole_client::DCClient;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DCClient::new("ws://127.0.0.1:9001").await?;

    // チャンネル別の処理関数を定義
    let mut channel_handlers = HashMap::new();

    // センサーデータチャンネル
    let sensor_channel = client.open("SensorData".to_string()).await?;
    let (sensor_tx, mut sensor_rx) = mpsc::channel(64);
    client.listen(sensor_channel, sensor_tx).await?;

    channel_handlers.insert(sensor_channel, "SensorData");

    // ログチャンネル
    let log_channel = client.open("Logs".to_string()).await?;
    let (log_tx, mut log_rx) = mpsc::channel(64);
    client.listen(log_channel, log_tx).await?;

    channel_handlers.insert(log_channel, "Logs");

    // センサーデータ処理
    tokio::spawn(async move {
        while let Some((channel_id, data)) = sensor_rx.recv().await {
            println!("🌡️  Sensor: {}", data);
            // センサーデータの解析処理をここに実装
        }
    });

    // ログ処理
    tokio::spawn(async move {
        while let Some((channel_id, data)) = log_rx.recv().await {
            println!("📝 Log: {}", data);
            // ログの保存処理をここに実装
        }
    });

    // テストデータを送信
    client.send(sensor_channel, "{\"temperature\": 25.5, \"humidity\": 60.2}".to_string()).await?;
    client.send(log_channel, "System started successfully".to_string()).await?;

    sleep(Duration::from_secs(5)).await;
    Ok(())
}
```

## 解説

### 1. 統合受信チャンネル
```rust
let (tx, mut rx) = mpsc::channel(256);
```
複数のDevConsoleチャンネルからのデータを統合して受信するためのmpscチャンネルを作成します。バッファサイズを大きめに設定しています。

### 2. 全チャンネルのリッスン
```rust
for (name, &channel_id) in &channels {
    client.listen(channel_id, tx.clone()).await?;
}
```
作成した全チャンネルで同じmpsc Senderを使用してリッスンを開始します。

### 3. チャンネル識別
```rust
let channel_name = channels.iter()
    .find(|(_, &id)| id == channel_id)
    .map(|(name, _)| name.as_str())
    .unwrap_or("Unknown");
```
受信したデータのチャンネルIDから、チャンネル名を逆引きして表示します。

### 4. 既存チャンネルの監視
```rust
let existing_channels = client.channel_list().await?;
```
他のクライアントが作成したチャンネルも検出して監視できます。

## 注意点

- 多数のチャンネルを監視する場合は、mpscチャンネルのバッファサイズを適切に設定してください
- チャンネル別に異なる処理を行いたい場合は、チャンネルIDに基づいて処理を分岐させてください
- `channel_list()`で取得できるのは現在存在するチャンネルのみで、リアルタイムでの新しいチャンネル通知機能はありません
