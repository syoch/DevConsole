# エラーハンドリング

この例では、DevConsoleクライアントでの適切なエラーハンドリングの実装方法を示します。

## 基本的なエラーハンドリング

```rust
use devconsole_client::{DCClient, DCClientError};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration, timeout};

#[tokio::main]
async fn main() {
    if let Err(e) = run_client().await {
        eprintln!("Application error: {}", e);
        std::process::exit(1);
    }
}

async fn run_client() -> Result<(), Box<dyn std::error::Error>> {
    // 接続エラーのハンドリング
    let mut client = match DCClient::new("ws://127.0.0.1:9001").await {
        Ok(client) => {
            println!("✅ Connected to DevConsole server");
            client
        }
        Err(e) => {
            eprintln!("❌ Failed to connect to server: {}", e);
            return Err(e.into());
        }
    };

    // チャンネル作成エラーのハンドリング
    let channel = match client.open("TestChannel".to_string()).await {
        Ok(channel) => {
            println!("✅ Created channel: {}", channel);
            channel
        }
        Err(DCClientError::WSError(e)) => {
            eprintln!("❌ WebSocket error during channel creation: {}", e);
            return Err(e.into());
        }
        Err(DCClientError::ConnectionBroken) => {
            eprintln!("❌ Connection broken during channel creation");
            return Err("Connection broken".into());
        }
    };

    // リッスンエラーのハンドリング
    let (tx, mut rx) = mpsc::channel(64);
    if let Err(e) = client.listen(channel, tx).await {
        match e {
            DCClientError::WSError(ws_err) => {
                eprintln!("❌ WebSocket error during listen: {}", ws_err);
                return Err(ws_err.into());
            }
            DCClientError::ConnectionBroken => {
                eprintln!("❌ Connection broken during listen");
                return Err("Connection broken".into());
            }
        }
    }

    println!("✅ Started listening to channel: {}", channel);

    // データ受信エラーのハンドリング
    tokio::spawn(async move {
        while let Some((channel_id, data)) = rx.recv().await {
            println!("📨 Received on channel {}: {}", channel_id, data);
        }
        println!("⚠️  Data receiver channel closed");
    });

    // データ送信エラーのハンドリング
    for i in 1..=5 {
        let message = format!("Test message {}", i);

        match client.send(channel, message.clone()).await {
            Ok(()) => {
                println!("📤 Sent: {}", message);
            }
            Err(DCClientError::WSError(e)) => {
                eprintln!("❌ Failed to send message: {}", e);
                // 送信エラーの場合は続行するか判断
                if is_fatal_error(&e) {
                    return Err(e.into());
                }
            }
            Err(DCClientError::ConnectionBroken) => {
                eprintln!("❌ Connection broken during send");
                return Err("Connection broken".into());
            }
        }

        sleep(Duration::from_secs(1)).await;
    }

    println!("✅ Demo completed successfully");
    Ok(())
}

fn is_fatal_error(error: &tokio_tungstenite::tungstenite::Error) -> bool {
    use tokio_tungstenite::tungstenite::Error;

    match error {
        Error::ConnectionClosed | Error::AlreadyClosed => true,
        Error::Io(_) => true,
        _ => false,
    }
}
```

## タイムアウト付きの操作

```rust
use tokio::time::{timeout, Duration};

async fn connect_with_timeout() -> Result<DCClient, Box<dyn std::error::Error>> {
    // 5秒でタイムアウト
    match timeout(Duration::from_secs(5), DCClient::new("ws://127.0.0.1:9001")).await {
        Ok(Ok(client)) => {
            println!("✅ Connected successfully");
            Ok(client)
        }
        Ok(Err(e)) => {
            eprintln!("❌ Connection failed: {}", e);
            Err(e.into())
        }
        Err(_) => {
            eprintln!("❌ Connection timed out");
            Err("Connection timeout".into())
        }
    }
}

async fn send_with_timeout(
    client: &mut DCClient,
    channel: u64,
    data: String,
) -> Result<(), Box<dyn std::error::Error>> {
    match timeout(Duration::from_secs(3), client.send(channel, data.clone())).await {
        Ok(Ok(())) => {
            println!("✅ Sent successfully: {}", data);
            Ok(())
        }
        Ok(Err(e)) => {
            eprintln!("❌ Send failed: {:?}", e);
            Err(format!("Send error: {:?}", e).into())
        }
        Err(_) => {
            eprintln!("❌ Send timed out");
            Err("Send timeout".into())
        }
    }
}
```

## 再接続機能付きクライアント

```rust
use tokio::time::{sleep, Duration};

struct RobustClient {
    url: String,
    client: Option<DCClient>,
    max_retries: usize,
}

impl RobustClient {
    fn new(url: String) -> Self {
        Self {
            url,
            client: None,
            max_retries: 3,
        }
    }

    async fn ensure_connected(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.client.is_some() {
            return Ok(());
        }

        for attempt in 1..=self.max_retries {
            println!("🔄 Connection attempt {} of {}", attempt, self.max_retries);

            match DCClient::new(&self.url).await {
                Ok(client) => {
                    self.client = Some(client);
                    println!("✅ Connected successfully");
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("❌ Connection attempt {} failed: {}", attempt, e);
                    if attempt < self.max_retries {
                        sleep(Duration::from_secs(2u64.pow(attempt as u32))).await; // 指数バックオフ
                    }
                }
            }
        }

        Err("Failed to connect after all retries".into())
    }

    async fn send_with_retry(&mut self, channel: u64, data: String) -> Result<(), Box<dyn std::error::Error>> {
        for attempt in 1..=self.max_retries {
            if let Err(_) = self.ensure_connected().await {
                continue;
            }

            if let Some(ref mut client) = self.client {
                match client.send(channel, data.clone()).await {
                    Ok(()) => return Ok(()),
                    Err(DCClientError::ConnectionBroken) => {
                        println!("⚠️  Connection broken, will retry");
                        self.client = None; // 再接続をトリガー
                    }
                    Err(e) => {
                        eprintln!("❌ Send error: {:?}", e);
                        return Err(e.into());
                    }
                }
            }

            if attempt < self.max_retries {
                sleep(Duration::from_secs(1)).await;
            }
        }

        Err("Failed to send after all retries".into())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut robust_client = RobustClient::new("ws://127.0.0.1:9001".to_string());

    // 接続とチャンネル作成
    robust_client.ensure_connected().await?;

    let channel = if let Some(ref mut client) = robust_client.client {
        client.open("RobustChannel".to_string()).await?
    } else {
        return Err("Client not connected".into());
    };

    // 耐障害性のあるデータ送信
    for i in 1..=10 {
        let message = format!("Robust message {}", i);
        match robust_client.send_with_retry(channel, message.clone()).await {
            Ok(()) => println!("📤 Sent: {}", message),
            Err(e) => eprintln!("❌ Failed to send {}: {}", message, e),
        }

        sleep(Duration::from_secs(2)).await;
    }

    Ok(())
}
```

## エラー分類と対処

```rust
fn handle_client_error(error: DCClientError) -> ErrorAction {
    match error {
        DCClientError::WSError(ws_error) => {
            use tokio_tungstenite::tungstenite::Error;
            match ws_error {
                Error::ConnectionClosed | Error::AlreadyClosed => {
                    println!("🔌 Connection closed - attempting reconnect");
                    ErrorAction::Reconnect
                }
                Error::Io(io_error) => {
                    eprintln!("🌐 Network error: {}", io_error);
                    ErrorAction::Reconnect
                }
                Error::Protocol(protocol_error) => {
                    eprintln!("📋 Protocol error: {}", protocol_error);
                    ErrorAction::Abort
                }
                Error::Utf8 => {
                    eprintln!("🔤 UTF-8 encoding error");
                    ErrorAction::Continue
                }
                _ => {
                    eprintln!("❓ Unknown WebSocket error: {}", ws_error);
                    ErrorAction::Abort
                }
            }
        }
        DCClientError::ConnectionBroken => {
            println!("💔 Connection broken - attempting reconnect");
            ErrorAction::Reconnect
        }
    }
}

enum ErrorAction {
    Continue,   // エラーを無視して続行
    Reconnect,  // 再接続を試行
    Abort,      // 処理を中止
}
```

## 実行方法

```bash
# 正常なサーバーがある場合
cargo run --bin devconsole_server &
cargo run --example error_handling

# サーバーを意図的に停止してエラー動作を確認
# Ctrl+Cでサーバーを停止し、クライアントの動作を観察
```

## 期待される出力

```
✅ Connected to DevConsole server
✅ Created channel: 1
✅ Started listening to channel: 1
📤 Sent: Test message 1
📨 Received on channel 1: Test message 1
📤 Sent: Test message 2
📨 Received on channel 1: Test message 2
...
✅ Demo completed successfully
```

## エラー時の出力例

```
❌ Failed to connect to server: Connection refused (os error 111)
🔄 Connection attempt 1 of 3
❌ Connection attempt 1 failed: Connection refused (os error 111)
🔄 Connection attempt 2 of 3
❌ Connection attempt 2 failed: Connection refused (os error 111)
🔄 Connection attempt 3 of 3
❌ Connection attempt 3 failed: Connection refused (os error 111)
Application error: Failed to connect after all retries
```

## ベストプラクティス

1. **適切なエラー分類**: 一時的なエラーと致命的なエラーを区別する
2. **タイムアウト設定**: 長時間のブロックを防ぐためにタイムアウトを設定
3. **再試行ロジック**: 指数バックオフを使用した再試行機能
4. **ログ出力**: エラーの詳細を適切にログ出力
5. **リソース管理**: エラー時のリソース解放を確実に行う
