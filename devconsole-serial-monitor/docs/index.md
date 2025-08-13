# DevConsole Serial Monitor

DevConsole Serial Monitorは、シリアルデバイス（USB/ACMデバイス）を自動検出し、受信したデータをDevConsoleチャンネルに送信するアプリケーションです。

## 機能概要

- シリアルデバイスの自動検出（`/dev/ttyACM*`, `/dev/ttyUSB*`）
- 複数デバイスの同時監視
- デバイスの接続・切断の動的検出
- 受信データのリアルタイム処理とDevConsoleチャンネルへの転送
- 文字エンコーディングと行単位での処理

## 起動方法

```bash
cargo run --bin devconsole_serial_monitor
```

アプリケーションは自動的にDevConsoleサーバー（`ws://localhost:9001`）に接続し、"SerialMonitor"という名前のチャンネルを作成します。

## 監視対象デバイス

以下のパターンにマッチするデバイスを自動検出します：

- `/dev/ttyACM*` - USB CDC ACMデバイス（Arduino、マイコンボードなど）
- `/dev/ttyUSB*` - USB-シリアル変換器

### デバイス検出の仕組み

1. **定期チェック**: 1秒間隔で`/dev`ディレクトリをスキャン
2. **新規検出**: 新しいデバイスを発見すると自動的に監視開始
3. **切断検出**: デバイスが削除されると監視を停止
4. **重複回避**: 既に監視中のデバイスは再度監視しない

## データフォーマット

シリアルデバイスから受信したデータは、以下のJSON形式でDevConsoleチャンネルに送信されます：

### デバイス接続時

```json
{
  "Opened": {
    "path": "/dev/ttyACM0"
  }
}
```

### データ受信時

```json
{
  "Line": {
    "path": "/dev/ttyACM0",
    "line": "受信したデータの内容"
  }
}
```

### デバイス切断時

```json
{
  "Closed": {
    "path": "/dev/ttyACM0"
  }
}
```

## シリアル通信設定

### 通信パラメータ

- **ボーレート**: 961200 bps
- **データビット**: 8
- **ストップビット**: 1
- **パリティ**: なし
- **フロー制御**: なし

### 設定変更

ボーレートを変更する場合は、`serial_monitor.rs`の以下の部分を編集してください：

```rust
let dev = SerialDevice::new(path.clone(), 961200); // ここでボーレートを指定
```

## データ処理

### 文字エンコーディング

受信したバイトデータは以下のルールで文字列に変換されます：

```rust
// 印字可能文字とエスケープ文字
if *ch == '\x1b' || (0x20u8 <= *ch as u8 && *ch as u8 <= 0x7e) {
    line.push(*ch);
} else {
    // その他の文字は16進数表記
    line.push_str(format!("\\x{:02X}", *ch as u8).as_str());
}
```

- **印字可能文字**: 0x20-0x7E（空白文字から~まで）はそのまま出力
- **エスケープ文字**: 0x1B（ESC）はそのまま出力
- **その他の文字**: `\xXX`形式（16進数表記）に変換

### 行単位処理

- **キャリッジリターン**: `\r`（0x0D）は無視
- **ラインフィード**: `\n`（0x0A）を受信すると行区切りとして処理
- **バッファリング**: 改行文字までのデータを内部バッファに蓄積
- **送信**: 改行を受信するとその行のデータを`Line`イベントとして送信

### データフロー

```
シリアルデバイス → バイトデータ → 文字変換 → 行バッファ → 改行検出 → JSON送信
```

## 内部アーキテクチャ

### コンポーネント構成

```
Serial Monitor
├── main.rs                    # メインループとDevConsole接続
├── device_watcher.rs          # デバイス検出
│   └── watcher_thread()      # /devディレクトリ監視
└── serial_monitor.rs          # シリアル通信
    └── monitor_thread()      # 個別デバイス監視
```

### スレッド構成

1. **メインスレッド**: DevConsole接続とイベント集約
2. **Device Watcherスレッド**: デバイス検出
3. **Serial Monitorスレッド**: 各デバイスに対して1つずつ作成

### イベントフロー

```
Device Watcher → DeviceFound → Main → spawn Serial Monitor
Serial Monitor → LineReceipt/Closed → Main → DevConsole
```

## 使用例

### DevConsoleでシリアルデータを受信

```rust
use devconsole_client::DCClient;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum SerialEvent {
    Opened { path: String },
    Line { path: String, line: String },
    Closed { path: String },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DCClient::new("ws://127.0.0.1:9001").await?;

    // SerialMonitorチャンネルを探す
    let channels = client.channel_list().await?;

    // 通常、SerialMonitorが作成したチャンネルは最初のチャンネル
    if let Some(&serial_channel) = channels.first() {
        let (tx, mut rx) = mpsc::channel(64);
        client.listen(serial_channel, tx).await?;

        println!("Listening to serial data on channel: {}", serial_channel);

        while let Some((_, data)) = rx.recv().await {
            match serde_json::from_str::<SerialEvent>(&data) {
                Ok(event) => match event {
                    SerialEvent::Opened { path } => {
                        println!("📱 Serial device connected: {}", path);
                    }
                    SerialEvent::Line { path, line } => {
                        println!("[{}] {}", path, line);
                    }
                    SerialEvent::Closed { path } => {
                        println!("📱 Serial device disconnected: {}", path);
                    }
                },
                Err(e) => {
                    eprintln!("Failed to parse serial event: {}", e);
                    println!("Raw data: {}", data);
                }
            }
        }
    } else {
        println!("No channels found. Make sure Serial Monitor is running.");
    }

    Ok(())
}
```

### マルチデバイス処理例

```rust
use std::collections::HashMap;

struct SerialDataCollector {
    devices: HashMap<String, Vec<String>>,
}

impl SerialDataCollector {
    fn new() -> Self {
        Self {
            devices: HashMap::new(),
        }
    }

    fn handle_event(&mut self, event: SerialEvent) {
        match event {
            SerialEvent::Opened { path } => {
                self.devices.insert(path.clone(), Vec::new());
                println!("Started collecting data from: {}", path);
            }
            SerialEvent::Line { path, line } => {
                if let Some(lines) = self.devices.get_mut(&path) {
                    lines.push(line.clone());
                    println!("[{}] Line #{}: {}", path, lines.len(), line);
                }
            }
            SerialEvent::Closed { path } => {
                if let Some(lines) = self.devices.remove(&path) {
                    println!("Device {} disconnected. Collected {} lines.", path, lines.len());
                }
            }
        }
    }
}
```

## トラブルシューティング

### 一般的な問題

1. **デバイスが検出されない**:
   - デバイスが`/dev/ttyACM*`または`/dev/ttyUSB*`として認識されているか確認
   - `ls /dev/tty*`でデバイスの存在を確認
   - デバイスのアクセス権限を確認（ユーザーがdialoutグループに所属している必要がある場合あり）

2. **接続できない**:
   - デバイスが他のアプリケーションで使用されていないか確認
   - ボーレートが正しく設定されているか確認

3. **データが受信されない**:
   - デバイス側がデータを送信しているか確認
   - シリアル通信の設定（ボーレート、データビットなど）が正しいか確認

4. **文字化け**:
   - ボーレートの不一致が原因の可能性
   - デバイス側の文字エンコーディングを確認

### デバッグ方法

1. **ログレベルの調整**:
```bash
RUST_LOG=debug cargo run --bin devconsole_serial_monitor
```

2. **手動デバイステスト**:
```bash
# デバイスの確認
ls -la /dev/ttyACM* /dev/ttyUSB*

# 手動でシリアル通信テスト（screenコマンド）
screen /dev/ttyACM0 961200
```

3. **DevConsoleサーバーのログ確認**:
別のターミナルでサーバーのログを確認してメッセージが送信されているか確認

## 権限設定

Linuxでシリアルデバイスにアクセスするには適切な権限が必要です：

```bash
# 現在のユーザーをdialoutグループに追加
sudo usermod -a -G dialout $USER

# 変更を反映するため再ログインまたは
newgrp dialout

# デバイスの権限確認
ls -la /dev/ttyACM0
```

## システム要件

- **OS**: Linux（`/dev`ディレクトリへのアクセスが必要）
- **権限**: シリアルデバイスへのアクセス権限
- **依存関係**: `srobo_base`ライブラリ（シリアル通信）
- **ネットワーク**: DevConsoleサーバーへのアクセス

## 設定のカスタマイズ

### サーバーURL変更

```rust
// main.rs の以下の行を変更
let mut client = devconsole_client::DCClient::new("ws://localhost:9001") // <- ここ
```

### チャンネル名変更

```rust
// main.rs の以下の行を変更
let channel = client.open("SerialMonitor".to_string()) // <- ここ
```

### 検出間隔変更

```rust
// device_watcher.rs の以下の行を変更
thread::sleep(Duration::from_secs(1)); // <- ここ
```

## パフォーマンス特性

- **検出遅延**: 最大1秒（デバイス検出間隔）
- **データ遅延**: リアルタイム（改行文字受信時に即座に送信）
- **メモリ使用量**: デバイス数と行バッファサイズに比例
- **CPU使用量**: デバイス数に比例（各デバイスに対して1スレッド）
