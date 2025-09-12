# DevConsole CLI

DevConsole CLI は、DevConsole サーバーと対話するためのコマンドラインツールです。

## ビルド

```bash
cargo build --bin devconsole_cli
```

## 使用方法

### 基本オプション

- `-s, --server <ADDRESS>`: DevConsole サーバーのアドレスを指定 (デフォルト: `ws://127.0.0.1:9001`)
- `-v, --verbose`: Node ID を表示
- `-h, --help`: ヘルプを表示

### コマンド

#### `list` - チャンネル一覧表示

利用可能なチャンネルの一覧を表示します。

```bash
./target/debug/devconsole_cli list
```

#### `info` - チャンネル情報表示

指定したチャンネルの詳細情報を表示します。

```bash
# チャンネル ID で指定
./target/debug/devconsole_cli info 1

# チャンネル名で指定
./target/debug/devconsole_cli info SerialMonitor
```

#### `open` - チャンネル作成

指定した名前で新しいチャンネルを作成します。

```bash
./target/debug/devconsole_cli open MyChannel
```

#### `send` - メッセージ送信

指定したチャンネルにメッセージを送信します。

```bash
# チャンネル ID で指定
./target/debug/devconsole_cli send 1 "Hello World"

# チャンネル名で指定
./target/debug/devconsole_cli send SerialMonitor "Test message"
```

#### `listen` - チャンネル監視

指定したチャンネルを監視し、受信したメッセージを標準出力に表示します。`Ctrl+C` で終了します。

```bash
# チャンネル ID で指定
./target/debug/devconsole_cli listen 1

# チャンネル名で指定
./target/debug/devconsole_cli listen SerialMonitor
```

### 例

```bash
# サーバーの情報を確認（Node ID表示）
./target/debug/devconsole_cli -v list

# 利用可能なチャンネルを表示
./target/debug/devconsole_cli list

# 新しいチャンネルを作成
./target/debug/devconsole_cli open TestChannel

# メッセージを送信
./target/debug/devconsole_cli send TestChannel "Hello from CLI!"

# チャンネルを監視
./target/debug/devconsole_cli listen TestChannel
```

## 機能

- ✅ コマンドライン引数の処理
- ✅ DevConsole サーバーのアドレス指定
- ✅ Node ID の表示 (`-v` オプション)
- ✅ チャンネルの監視とメッセージ受信表示
- ✅ チャンネルへのメッセージ送信
- ✅ 利用可能なチャンネル一覧の表示
- ✅ チャンネルの作成
- ✅ チャンネル情報の表示
- ✅ チャンネル名とID両方による指定のサポート
