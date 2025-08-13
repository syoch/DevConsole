# DevConsole

DevConsoleは、複数のクライアントが接続してチャンネルベースのデータ通信を行うRustベースのシステムです。WebSocketプロトコルを使用してリアルタイムでデータの送受信を行います。

## システム概要

DevConsoleは以下のコンポーネントで構成されています：

- **Server**: WebSocketサーバーとして動作し、クライアント間の通信を仲介
- **Client Library**: サーバーに接続してチャンネルの作成・リッスン・データ送信を行うライブラリ
- **Protocol**: サーバーとクライアント間の通信プロトコル定義
- **Serial Monitor**: シリアルデバイスを監視してDevConsoleチャンネルにデータを送信
- **Data Logger**: DevConsoleチャンネルからデータを受信してログ記録

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

## プロジェクト構造

```
DevConsole/
├── docs/                         # 📚 プロジェクト全体のドキュメント
│   └── index.md                 # システム概要とアーキテクチャ
├── devconsole-server/           # 🖥️ WebSocketサーバー
│   └── docs/index.md           # サーバーの仕様と設定
├── devconsole-client/           # 📡 クライアントライブラリ
│   └── docs/                   # クライアント固有ドキュメント
│       ├── index.md           # ライブラリ使用方法
│       └── examples/          # 使用例
├── devconsole-protocol/         # 📋 プロトコル定義
│   └── docs/index.md          # プロトコルライブラリ使用方法
├── devconsole-serial-monitor/   # 📱 シリアルデバイス監視アプリ
│   └── docs/index.md          # Serial Monitor使用方法
└── devconsole-data-logger/      # 📊 データロガーアプリ
    └── docs/index.md          # Data Logger使用方法
```

## ドキュメント

### 📚 [システム全体ドキュメント](./docs/index.md)
- アーキテクチャ概要
- コンポーネント間の関係
- 基本的な使用フロー

### 🖥️ [DevConsole Server](./devconsole-server/docs/index.md)
- サーバーの機能と設定
- 起動方法と設定変更
- ログ出力とトラブルシューティング

### 📡 [DevConsole Client](./devconsole-client/docs/index.md)
- クライアントAPIリファレンス
- 基本的な使用方法
- エラーハンドリング

### 📋 [DevConsole Protocol](./devconsole-protocol/docs/index.md)
- 通信プロトコルの詳細仕様
- イベント定義とデータ型
- JSON形式の例

### 📱 [Serial Monitor](./devconsole-serial-monitor/docs/index.md)
- シリアルデバイス監視の使用方法
- データフォーマットと設定
- トラブルシューティング

### 📊 [Data Logger](./devconsole-data-logger/docs/index.md)
- 全チャンネル監視の使用方法
- ログ出力のカスタマイズ
- 活用場面と例

## 開発環境

### システム要件

- **Rust**: 1.88.0以上
- **OS**: Linux（Serial Monitorを使用する場合）
- **その他**: Tokio非同期ランタイム

### 開発環境のセットアップ

```bash
# Nix環境の場合
nix develop

# 手動セットアップの場合
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### ビルドと実行

```bash
# 全コンポーネントのビルド
cargo build

# 個別コンポーネントの実行
cargo run --bin devconsole_server
cargo run --bin devconsole_serial_monitor
cargo run --bin devconsole_data_logger

# テストの実行
cargo test
```

## 基本的な使用フロー

1. **サーバー起動**: DevConsole Serverを起動（ポート9001でリッスン）
2. **アプリケーション起動**: Serial Monitor、Data Logger、またはカスタムクライアントを起動
3. **チャンネル作成**: アプリケーションが必要に応じてチャンネルを作成
4. **データ通信**:
   - アプリケーションが`listen()`でチャンネルをリッスン
   - アプリケーションが`send()`でデータを送信
5. **切断処理**: クライアントが切断されると、そのクライアントが作成したチャンネルは自動削除

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

## ライセンス

このプロジェクトのライセンス情報については、適切なライセンスファイルを参照してください。

## 貢献

このプロジェクトへの貢献を歓迎します。バグ報告、機能要望、プルリクエストなどをお待ちしています。

## サポート

問題が発生した場合：

1. [ドキュメント](./docs/index.md)を確認
2. 各コンポーネントの個別ドキュメントを参照
3. ログを確認してエラーの詳細を特定
4. 必要に応じてIssueを作成
