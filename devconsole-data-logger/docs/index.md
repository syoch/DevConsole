# DevConsole Data Logger

DevConsole Data Loggerは、DevConsoleサーバーの全チャンネルを自動的に監視し、受信したデータをログ出力するアプリケーションです。

## 機能概要

- DevConsoleサーバーへの自動接続
- 利用可能な全チャンネルの自動検出
- 新しいチャンネルの動的検出と監視開始
- 受信データのリアルタイムログ出力
- 継続的なチャンネル監視

## 起動方法

```bash
cargo run --bin devconsole_data_logger
```

アプリケーションは自動的にDevConsoleサーバー（`ws://127.0.0.1:9001`）に接続し、監視を開始します。

## 動作フロー

1. **初期化**: DevConsoleサーバー（`ws://127.0.0.1:9001`）に接続
2. **チャンネル検出**: `channel_list()`で利用可能なチャンネル一覧を取得
3. **監視開始**: 検出した各チャンネルの`listen()`を開始
4. **定期チェック**: 5秒間隔で新しいチャンネルを検出
5. **ログ出力**: 受信したデータを標準出力にログ出力

## ログフォーマット

受信したデータは以下の形式でログ出力されます：

```
[INFO] Received data on channel {channel_id}: {data}
```

### 実際の出力例

```
[DEBUG] Connected to DevConsole server
[DEBUG] Found channel: 1
[DEBUG] Started listening to channel: 1
[INFO] Received data on channel 1: {"Opened":{"path":"/dev/ttyACM0"}}
[INFO] Received data on channel 1: {"Line":{"path":"/dev/ttyACM0","line":"Temperature: 23.5°C"}}
[INFO] Received data on channel 1: {"Line":{"path":"/dev/ttyACM0","line":"Humidity: 58.2%"}}
[DEBUG] Found channel: 2
[DEBUG] Started listening to channel: 2
[INFO] Received data on channel 2: {"sensor_id": "temp_01", "value": 24.1}
```

## 設定とカスタマイズ

### 接続先サーバー

デフォルトの接続先は`ws://127.0.0.1:9001`です。変更する場合は、`main.rs`の以下の部分を編集してください：

```rust
let mut client = devconsole_client::DCClient::new("ws://127.0.0.1:9001")
    .await
    .unwrap();
```

### チェック間隔

新しいチャンネルの検出間隔はデフォルトで5秒です。変更する場合は以下の部分を編集してください：

```rust
sleep(Duration::from_secs(5)).await; // 5秒間隔
```

より頻繁にチェックしたい場合：

```rust
sleep(Duration::from_secs(1)).await; // 1秒間隔
```

### ログレベル

デフォルトのログレベルはDebugです。変更する場合は以下の部分を編集してください：

```rust
logger::Builder::new()
    .filter(None, log::LevelFilter::Debug)  // Info, Warn, Error等に変更可能
    .init();
```

利用可能なログレベル：
- `Error`: エラーのみ
- `Warn`: 警告以上
- `Info`: 情報以上（推奨）
- `Debug`: デバッグ情報も含む（デフォルト）
- `Trace`: 全ての情報

## 内部実装

### アーキテクチャ

```
Data Logger
├── main.rs                    # メインループ
├── DCClient                   # DevConsole接続
├── listening_channels         # 監視中チャンネルリスト
└── Data Receiver Task         # データ受信タスク
    └── mpsc::channel         # データ受信チャンネル
```

### データフロー

```
1. Channel List Request → Server
2. Server → Channel List Response
3. For each new channel:
   - Listen Request → Server
   - Server → Data → Data Logger → Log Output
4. Sleep 5 seconds
5. Repeat from step 1
```

### メモリ管理

- **チャンネルリスト**: 監視中のチャンネルIDをVecで管理
- **データバッファ**: mpscチャンネルのバッファサイズは64
- **重複回避**: 既に監視中のチャンネルは再度監視しない

## 使用例

### 基本的な使用方法

```bash
# Terminal 1: DevConsole Serverを起動
cargo run --bin devconsole_server

# Terminal 2: Data Loggerを起動
cargo run --bin devconsole_data_logger
```

### 他のアプリケーションと組み合わせた使用

```bash
# Terminal 1: DevConsole Serverを起動
cargo run --bin devconsole_server &

# Terminal 2: Serial Monitorを起動（シリアルデータを送信）
cargo run --bin devconsole_serial_monitor &

# Terminal 3: Data Loggerを起動（全データを監視）
cargo run --bin devconsole_data_logger
```

この構成により、Serial Monitorが収集したシリアルデータがData Loggerでリアルタイムに確認できます。

### 出力のリダイレクト

```bash
# ファイルにログを保存
cargo run --bin devconsole_data_logger > data_log.txt

# 日付付きファイルに保存
cargo run --bin devconsole_data_logger > "data_log_$(date +%Y%m%d_%H%M%S).txt"

# リアルタイム表示とファイル保存の両方
cargo run --bin devconsole_data_logger | tee data_log.txt
```

### JSON形式データの解析

受信したデータがJSON形式の場合、より詳細な解析が可能です：

```rust
// カスタムData Loggerの例
use serde_json::Value;

fn parse_and_log_data(channel: u64, data: String) {
    match serde_json::from_str::<Value>(&data) {
        Ok(json) => {
            println!("[Channel {}] Parsed JSON: {:#}", channel, json);

            // 特定のフィールドを抽出
            if let Some(line) = json.get("Line") {
                if let (Some(path), Some(line_data)) =
                   (line.get("path"), line.get("line")) {
                    println!("Serial [{}]: {}", path, line_data);
                }
            }
        }
        Err(_) => {
            println!("[Channel {}] Raw data: {}", channel, data);
        }
    }
}
```

## エラーハンドリング

### 接続エラー

アプリケーションはサーバー接続に失敗した場合、`unwrap()`でパニックします：

```rust
let mut client = devconsole_client::DCClient::new("ws://127.0.0.1:9001")
    .await
    .unwrap(); // 接続失敗時はパニック
```

### チャンネルリッスンエラー

特定のチャンネルのリッスンに失敗した場合は、そのチャンネルをスキップして続行します：

```rust
if !listening_channels.contains(&channel) {
    match client.listen(channel, tx.clone()).await {
        Ok(()) => {
            listening_channels.push(channel);
            // 成功ログ
        }
        Err(e) => {
            eprintln!("Failed to listen to channel {}: {:?}", channel, e);
            // エラーログを出力して続行
        }
    }
}
```

### データ受信エラー

データ受信タスク内でエラーが発生した場合の処理：

```rust
tokio::spawn(async move {
    while let Some((channel, data)) = rx.recv().await {
        match std::panic::catch_unwind(|| {
            info!("Received data on channel {}: {}", channel, data);
        }) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Error processing data: {:?}", e);
            }
        }
    }
    warn!("Data receiver task ended");
});
```

## パフォーマンス特性

- **チャンネル検出遅延**: 最大5秒
- **データ受信遅延**: リアルタイム
- **メモリ使用量**: チャンネル数とデータ量に比例
- **CPU使用量**: 受信データ量に比例

### 大量データ処理

大量のデータが流れる場合の考慮事項：

1. **バッファサイズの調整**:
```rust
let (tx, mut rx) = mpsc::channel(1024); // バッファサイズを増加
```

2. **ログレベルの調整**:
```rust
logger::Builder::new()
    .filter(None, log::LevelFilter::Info) // DEBUGログを無効化
    .init();
```

3. **ファイル出力の使用**:
```bash
cargo run --bin devconsole_data_logger > data.log 2>&1
```

## トラブルシューティング

### 一般的な問題

1. **接続できない**:
   - DevConsoleサーバーが起動しているか確認
   - ポート9001が使用可能か確認
   - ファイアウォール設定を確認

2. **チャンネルが検出されない**:
   - 他のアプリケーション（Serial Monitorなど）が起動しているか確認
   - サーバーのログで`ChannelListRequest`が処理されているか確認

3. **データが表示されない**:
   - チャンネルにデータが送信されているか確認
   - ログレベルがInfoまたはDebugに設定されているか確認

### デバッグ手順

1. **詳細ログの有効化**:
```bash
RUST_LOG=debug cargo run --bin devconsole_data_logger
```

2. **サーバーログの確認**:
別のターミナルでサーバーのログを確認

3. **チャンネル一覧の手動確認**:
```rust
// デバッグ用コードを追加
let channels = client.channel_list().await.unwrap();
println!("Available channels: {:?}", channels);
```

## 活用場面

### 開発・デバッグ
- システム全体のデータフローの確認
- リアルタイムでのデータ監視
- 問題発生時のデータトレース

### 監視・ログ
- 全チャンネルのデータを中央集約してログ記録
- 長時間運用でのデータ収集
- 異常データの検出

### データ解析
- 受信データをファイルにリダイレクトして後で解析
- データの統計分析
- パフォーマンス測定

### システム統合
- 他のログシステムとの連携
- 監視システムへのデータ転送
- アラート機能との組み合わせ

## カスタマイズ例

### フィルタリング機能付きData Logger

```rust
use regex::Regex;

struct FilteredDataLogger {
    channel_filters: HashMap<u64, Regex>,
}

impl FilteredDataLogger {
    fn should_log(&self, channel: u64, data: &str) -> bool {
        if let Some(filter) = self.channel_filters.get(&channel) {
            filter.is_match(data)
        } else {
            true // フィルターがない場合は全て表示
        }
    }
}
```

### タイムスタンプ付きログ

```rust
use chrono::{DateTime, Utc};

fn log_with_timestamp(channel: u64, data: String) {
    let timestamp: DateTime<Utc> = Utc::now();
    println!("[{}] [Channel {}] {}",
             timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
             channel,
             data);
}
```
