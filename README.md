# Serial Monitor

シリアルデバイスの監視とログ出力を行うプログラムです。

## ディレクトリ構造

```
99-serial-monitor/
├── CMakeLists.txt          # ビルド設定
├── main.cpp               # メインエントリーポイント
├── dump-serial.cpp        # シリアルモニターの出力をJSON形式でダンプするツール
├── websocket-server.cpp   # WebSocketサーバー（プロトコルメッセージをJSON形式で配信）
├── test_websocket_client.html # WebSocketクライアントのテスト用HTMLページ
├── monitor.html           # マルチデバイス監視ダッシュボード（xterm.js対応）
├── include/               # 公開ヘッダーファイル
│   ├── constants.hpp      # 共通定数定義
│   └── serial_monitor.hpp # シリアルモニターのコア機能
├── src/                   # 内部実装ファイル
│   ├── core/              # コア機能
│   │   ├── polling_fd.hpp        # ファイルディスクリプタのポーリング
│   │   ├── file_poller.hpp       # ファイル監視機能
│   │   └── serial_device_watcher.hpp # シリアルデバイス監視メイン
│   ├── handlers/          # ハンドラー実装
│   │   ├── console_handler.hpp      # コンソール出力ハンドラー
│   │   └── sock_server_handler.hpp  # UNIXソケットサーバーハンドラー
│   ├── protocol/          # プロトコル関連
│   │   ├── monitor_protocol.hpp     # モニタープロトコル実装
│   │   └── protocol_constants.hpp   # プロトコル定数定義
│   ├── websocket/         # WebSocket機能
│   │   ├── websocket_server_simple.hpp # WebSocketサーバー実装
│   │   └── websocket_server_simple.cpp
│   └── crypto/            # 暗号化機能（WebSocket用）
│       ├── sha1.hpp       # SHA1ハッシュ実装
│       ├── sha1.cpp
│       ├── base64.hpp     # Base64エンコード/デコード
│       └── base64.cpp
└── docs/                  # ドキュメント
    └── protocol.md        # プロトコル仕様書
```

## ビルド方法

```bash
# プロジェクトルートから
mkdir -p build && cd build
cmake ..
make 99-serial-monitor 99-dump-serial 99-websocket-server
```

## 使用方法

### Serial Monitor

```bash
./99-serial-monitor
```

シリアルデバイスを自動検出し、入力をログ出力します。

### Dump Serial

```bash
./99-dump-serial
```

Serial Monitor からの出力をJSON形式でダンプします。

### WebSocket Server

```bash
./99-websocket-server
```

Serial Monitor からの出力をWebSocket経由でJSON形式で配信します。
デフォルトでポート8081で動作し、Webブラウザからリアルタイムでシリアルデータを監視できます。

テスト用HTMLクライアント: `test_websocket_client.html`

### Monitor Dashboard

```
monitor.html
```

マルチデバイス監視ダッシュボード：
- 複数のシリアルデバイスを同時に監視
- デバイスごとの画面分割表示
- xterm.js によるエスケープシーケンス対応
- コンパクトな文字列表示（隙間なし）
- アーカイブ機能（切断されたデバイスの履歴保存）
- アーカイブからの復活機能
- 接続時刻、切断時刻、メッセージ数の記録

## アーキテクチャ

- **Core**: デバイス監視とシリアル通信の基本機能
- **Handlers**: 異なる出力形式に対応するハンドラー
- **Protocol**: ソケット通信プロトコルの実装
- **WebSocket**: WebSocket経由でのリアルタイムデータ配信
- **Crypto**: WebSocketハンドシェイクに必要な暗号化機能（SHA1、Base64）
- **Constants**: プロジェクト全体で使用する共通定数

各レイヤーは明確に分離されており、拡張性と保守性を考慮した設計になっています。

## JSON形式

WebSocketサーバーは、dump-serial.cppと同じ形式でプロトコルメッセージをJSON形式に変換します：

```json
{
  "timestamp": 1234567890123,
  "type": "string_input",
  "data": {
    "device_name": "/dev/ttyUSB0",
    "lines": ["Hello", "World"]
  }
}
```

対応するメッセージタイプ:
- `sync`: 同期メッセージ
- `ping`/`pong`: 疎通確認
- `device_connect`/`device_disconnect`: デバイス接続状態
- `string_input`: 文字列入力データ
- `unknown`: 未知のメッセージタイプ
- `error`: 解析エラー
