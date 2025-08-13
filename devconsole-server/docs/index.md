# DevConsole Server

DevConsole Serverは、WebSocketサーバーとして動作し、複数のクライアントの接続を管理してチャンネルベースのメッセージ配信を行います。

## 機能概要

- WebSocket接続の受付（デフォルトポート：9001）
- ノードIDの自動割り当て
- チャンネルの管理と作成
- メッセージのブロードキャスト配信
- クライアント切断時のリソース自動清理

## 起動方法

```bash
cargo run --bin devconsole_server
```

サーバーは`127.0.0.1:9001`でWebSocket接続を待機します。

## 主要機能

### ノード管理

- 各クライアント接続に対して一意のノードIDを自動生成・割り当て
- ノードIDは接続時に`NodeIDNotification`イベントでクライアントに通知
- ノードIDは1から開始して順次インクリメント

### チャンネル管理

- **チャンネル作成**: クライアントからの`ChannelOpenRequest`に応じて新しいチャンネルを作成
- **チャンネル情報**: 作成されたチャンネルの名前、チャンネルID、作成者ノードIDを管理
- **チャンネル一覧**: 現在利用可能なチャンネルの一覧をクライアントに提供
- **自動削除**: ノードが切断されると、そのノードが作成したチャンネルを自動削除

### メッセージ配信

- **リッスン管理**: 各クライアントがリッスンしているチャンネルを追跡
- **ブロードキャスト**: チャンネルに送信されたデータを、そのチャンネルをリッスンしている全クライアントに配信
- **効率的配信**: リッスンしていないクライアントにはデータを送信しない

## 対応イベント

### 受信イベント（クライアント → サーバー）

- `ChannelOpenRequest`: 新しいチャンネルの作成要求
- `ChannelListenRequest`: チャンネルのリッスン開始要求
- `ChannelCloseRequest`: チャンネルの閉鎖要求
- `ChannelListRequest`: チャンネル一覧の取得要求
- `ChannelInfoRequest`: チャンネル詳細情報の取得要求
- `Data`: チャンネルへのデータ送信

### 送信イベント（サーバー → クライアント）

- `NodeIDNotification`: 接続時のノードID通知
- `ChannelOpenResponse`: チャンネル作成結果の応答
- `ChannelListenResponse`: リッスン開始結果の応答
- `ChannelListResponse`: チャンネル一覧の応答
- `ChannelInfoResponse`: チャンネル詳細情報の応答
- `Data`: リッスン中のクライアントへのデータ配信

## アーキテクチャ

### コンポーネント構成

```
DevConsole Server
├── Server (SharedServer)        # サーバー状態管理
│   ├── IDManager<NodeID>       # ノードID管理
│   ├── IDManager<ChannelID>    # チャンネルID管理
│   ├── Vec<Channel>            # チャンネル情報
│   └── Vec<SharedClient>       # 接続クライアント
├── Client (SharedClient)        # クライアント状態管理
│   ├── WebSocket Writer        # 送信用WebSocketストリーム
│   ├── NodeID                  # クライアントのノードID
│   └── Vec<ChannelID>         # リッスン中チャンネル
└── Channel                     # チャンネル情報
    ├── ChannelID              # チャンネルID
    ├── name: String           # チャンネル名
    └── supplied_by: NodeID    # 作成者ノードID
```

### データフロー

1. **接続確立**:
   ```
   Client → WebSocket接続 → Server
   Server → NodeIDNotification → Client
   ```

2. **チャンネル作成**:
   ```
   Client → ChannelOpenRequest → Server
   Server → 新しいチャンネル作成
   Server → ChannelOpenResponse → Client
   ```

3. **データ送信**:
   ```
   Client → Data → Server
   Server → リッスン中クライアント検索
   Server → Data → 各リッスンクライアント
   ```

## 設定とカスタマイズ

### 接続設定

現在のサーバー設定：

- **ポート**: 9001
- **アドレス**: 127.0.0.1 (ローカルホストのみ)
- **ログレベル**: Debug

設定変更は、`main.rs`の以下の部分を編集してください：

```rust
// サーバーアドレスとポートの変更
let tcp_server = TcpListener::bind("127.0.0.1:9001").await.unwrap();

// ログレベルの変更
logger::Builder::new()
    .filter(None, log::LevelFilter::Debug)  // Info, Warn, Error等に変更可能
    .init();
```

### 外部アクセスの許可

ローカルホスト以外からのアクセスを許可する場合：

```rust
// すべてのインターフェースで待機
let tcp_server = TcpListener::bind("0.0.0.0:9001").await.unwrap();
```

**注意**: セキュリティ上の理由により、外部アクセスを許可する場合は適切なファイアウォール設定を行ってください。

## ログ出力

サーバーは以下の情報をログ出力します：

### Debug レベル
- クライアントの接続・切断情報
- 受信イベントの詳細
- チャンネル作成・削除の詳細

### Info レベル
- チャンネル閉鎖要求の受信

### Error レベル
- 処理されないイベントに関する警告
- WebSocketエラー
- 不明なチャンネルへのアクセス

### ログ出力例

```
[DEBUG] New client connected, assigned node ID: 123
[DEBUG] Channel created: 1 (TestChannel) by node 123
[DEBUG] Node 123 started listening to channel 1
[INFO] Received ChannelCloseRequest for channel 1
[ERROR] ChannelInfoRequest for unknown channel 999
[DEBUG] Node 123 disconnected, cleaning up channels
```

## パフォーマンス特性

- **同時接続数**: 理論的にはTokioの制限まで（通常数千から数万）
- **メッセージ配信**: O(n) - nはリッスンしているクライアント数
- **チャンネル管理**: O(1) - チャンネル作成・削除
- **メモリ使用量**: 接続数とチャンネル数に比例

## 制限事項

- **認証機能**: 現在は認証機能がありません
- **データ永続化**: サーバー再起動時にチャンネル情報は失われます
- **データサイズ制限**: WebSocketメッセージサイズの制限のみ
- **レート制限**: 現在はメッセージレート制限がありません

## 開発とテスト

### ローカル開発

```bash
# サーバー起動
cargo run --bin devconsole_server

# 別のターミナルでクライアントのテスト
cargo run --bin devconsole_data_logger
```

### ログレベルの調整

開発時により詳細なログが必要な場合：

```bash
RUST_LOG=debug cargo run --bin devconsole_server
```

### WebSocketクライアントでのテスト

WebSocketクライアントツールを使用してサーバーをテストできます：

```bash
# WebSocketクライアントツール例（wscat）
wscat -c ws://127.0.0.1:9001

# メッセージ送信例
{"ChannelOpenRequest": {"name": "TestChannel"}}
```

## トラブルシューティング

### 一般的な問題

1. **接続できない**:
   - ポート9001が他のプロセスで使用されていないか確認
   - ファイアウォールの設定を確認

2. **チャンネルが作成されない**:
   - JSON形式が正しいか確認
   - サーバーログでエラーメッセージを確認

3. **データが受信されない**:
   - `ChannelListenRequest`が正常に送信されているか確認
   - チャンネルIDが正しいか確認

### デバッグ手順

1. サーバーログでエラーメッセージを確認
2. WebSocketメッセージの形式を確認
3. チャンネル一覧でチャンネルの存在を確認
4. 必要に応じてログレベルをDebugに設定
