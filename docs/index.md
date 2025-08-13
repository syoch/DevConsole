# DevConsole

DevConsoleは、複数のクライアントが接続してチャンネルベースのデータ通信を行うシステムです。WebSocketプロトコルを使用してリアルタイムでデータの送受信を行います。

## システム概要

DevConsoleは以下のコンポーネントで構成されています：

- **Server**: WebSocketサーバーとして動作し、クライアント間の通信を仲介
- **Client**: サーバーに接続してチャンネルの作成・リッスン・データ送信を行うライブラリ
- **Protocol**: サーバーとクライアント間の通信プロトコル定義
- **Serial Monitor**: シリアルデバイスを監視してDevConsoleチャンネルにデータを送信
- **Data Logger**: DevConsoleチャンネルからデータを受信してログ記録

## アーキテクチャ

```
┌─────────────────┐    WebSocket    ┌──────────────────┐
│  DevConsole     │◄───────────────►│  DevConsole      │
│  Client         │                 │  Server          │
│                 │                 │                  │
│  - listen()     │                 │  - チャンネル管理  │
│  - send()       │                 │  - データブロード │
│  - open()       │                 │  - ノード管理     │
└─────────────────┘                 └──────────────────┘
         ▲                                    ▲
         │                                    │
         │                                    │
┌─────────────────┐                 ┌──────────────────┐
│  Serial Monitor │                 │  Data Logger     │
│  Application    │                 │  Application     │
└─────────────────┘                 └──────────────────┘
```

## コンポーネント一覧

### [DevConsole Server](../devconsole-server/docs/index.md)
WebSocketサーバーとして動作し、クライアント接続の管理とチャンネルベースのメッセージ配信を担当します。

### [DevConsole Protocol](../devconsole-protocol/docs/index.md)
サーバーとクライアント間の通信で使用されるイベント定義とデータ型を提供します。

### [DevConsole Client](../devconsole-client/docs/index.md)
DevConsoleサーバーに接続するためのライブラリです。チャンネルの作成、リッスン、データ送信機能を提供します。

### [Serial Monitor](../devconsole-serial-monitor/docs/index.md)
シリアルデバイス（USB/ACMデバイス）を自動検出し、受信データをDevConsoleチャンネルに送信するアプリケーションです。

### [Data Logger](../devconsole-data-logger/docs/index.md)
DevConsoleの全チャンネルを監視し、受信したデータをログ出力するアプリケーションです。

## クイックスタート

### 1. サーバーの起動

```bash
cargo run --bin devconsole_server
```

### 2. Serial Monitorの起動（シリアルデバイス監視）

```bash
cargo run --bin devconsole_serial_monitor
```

### 3. Data Loggerの起動（全チャンネル監視）

```bash
cargo run --bin devconsole_data_logger
```

### 4. クライアントライブラリの使用

```rust
use devconsole_client::DCClient;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DCClient::new("ws://127.0.0.1:9001").await?;
    let channel = client.open("MyChannel".to_string()).await?;

    let (tx, mut rx) = mpsc::channel(64);
    client.listen(channel, tx).await?;

    client.send(channel, "Hello, DevConsole!".to_string()).await?;

    if let Some((ch, data)) = rx.recv().await {
        println!("Received: {}", data);
    }

    Ok(())
}
```

## 基本的な使用フロー

1. **サーバー起動**: DevConsole Serverを起動（ポート9001でリッスン）
2. **クライアント接続**: アプリケーションがDCClientを使用してサーバーに接続
3. **チャンネル作成**: `open()`でチャンネルを作成
4. **データ通信**:
   - `listen()`でチャンネルをリッスン
   - `send()`でデータを送信
5. **切断**: クライアントが切断されると、そのクライアントが作成したチャンネルは削除

## 開発環境

このプロジェクトはRustで開発されており、以下の環境で動作します：

- Rust 1.88.0
- Tokio (非同期ランタイム)
- WebSocket (tokio-tungstenite)

開発環境のセットアップについては、Nix flakeが提供されています。

### ビルドと実行

```bash
# 全コンポーネントのビルド
cargo build

# 個別コンポーネントの実行
cargo run --bin devconsole_server
cargo run --bin devconsole_serial_monitor
cargo run --bin devconsole_data_logger
```

## プロジェクト構造

```
DevConsole/
├── docs/                         # プロジェクト全体のドキュメント
│   └── index.md                 # このファイル
├── devconsole-server/           # WebSocketサーバー
│   └── docs/index.md           # サーバー関連ドキュメント
├── devconsole-client/           # クライアントライブラリ
│   └── docs/                   # クライアント固有ドキュメント
│       ├── index.md           # ライブラリ使用方法
│       └── examples/          # 使用例
├── devconsole-protocol/         # プロトコル定義
│   └── docs/index.md          # プロトコル関連ドキュメント
├── devconsole-serial-monitor/   # シリアルデバイス監視アプリ
│   └── docs/index.md          # Serial Monitor関連ドキュメント
└── devconsole-data-logger/      # データロガーアプリ
    └── docs/index.md          # Data Logger関連ドキュメント
```
