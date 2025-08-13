# 基本的な送受信

この例では、DevConsoleクライアントの基本的な使用方法を示します。

## コード例

```rust
use devconsole_client::DCClient;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. サーバーに接続
    let mut client = DCClient::new("ws://127.0.0.1:9001").await?;
    println!("Connected to DevConsole server");

    // 2. 新しいチャンネルを作成
    let channel = client.open("TestChannel".to_string()).await?;
    println!("Created channel: {}", channel);

    // 3. データ受信の準備
    let (tx, mut rx) = mpsc::channel(64);
    client.listen(channel, tx).await?;
    println!("Started listening to channel: {}", channel);

    // 4. データ受信の処理（バックグラウンド）
    tokio::spawn(async move {
        while let Some((channel_id, data)) = rx.recv().await {
            println!("Received on channel {}: {}", channel_id, data);
        }
    });

    // 5. データを送信（複数回）
    for i in 1..=5 {
        let message = format!("Message #{}", i);
        client.send(channel, message.clone()).await?;
        println!("Sent: {}", message);

        // 1秒待機
        sleep(Duration::from_secs(1)).await;
    }

    // 6. 最後のメッセージの処理を待つ
    sleep(Duration::from_secs(2)).await;

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
cargo run --example basic
```

## 期待される出力

```
Connected to DevConsole server
Created channel: 1
Started listening to channel: 1
Sent: Message #1
Received on channel 1: Message #1
Sent: Message #2
Received on channel 1: Message #2
Sent: Message #3
Received on channel 1: Message #3
Sent: Message #4
Received on channel 1: Message #4
Sent: Message #5
Received on channel 1: Message #5
Demo completed
```

## 解説

### 1. 接続の確立
```rust
let mut client = DCClient::new("ws://127.0.0.1:9001").await?;
```
DevConsoleサーバーへのWebSocket接続を確立します。エラーが発生した場合は接続に失敗します。

### 2. チャンネルの作成
```rust
let channel = client.open("TestChannel".to_string()).await?;
```
"TestChannel"という名前の新しいチャンネルを作成し、チャンネルIDを取得します。

### 3. リッスンの開始
```rust
let (tx, mut rx) = mpsc::channel(64);
client.listen(channel, tx).await?;
```
作成したチャンネルからのデータ受信を開始します。受信したデータはmpscチャンネル経由で通知されます。

### 4. 非同期データ受信
```rust
tokio::spawn(async move {
    while let Some((channel_id, data)) = rx.recv().await {
        println!("Received on channel {}: {}", channel_id, data);
    }
});
```
別のタスクでデータ受信処理を行い、メインタスクが他の処理を続行できるようにします。

### 5. データ送信
```rust
client.send(channel, message.clone()).await?;
```
作成したチャンネルにデータを送信します。送信されたデータは、そのチャンネルをリッスンしている全てのクライアント（この場合は自分自身）に配信されます。

## 注意点

- `client.listen()`を呼び出してから`client.send()`でデータを送信すると、自分が送信したデータも受信することになります
- データ受信処理は別のタスクで実行することで、メインタスクがブロックされることを防げます
- エラーハンドリングでは`?`演算子を使用して、エラーが発生した場合に早期リターンしています
