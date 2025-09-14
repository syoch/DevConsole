use clap::{Arg, ArgMatches, Command};
use devconsole::{ChannelID, DCClient};
use log::error;
use std::io::{self, Write};
use tokio::{select, sync::mpsc};

#[tokio::main]
pub async fn main() {
    env_logger::Builder::new()
        .filter(None, log::LevelFilter::Info)
        .format_timestamp(None)
        .format_module_path(false)
        .format_target(false)
        .init();

    let matches = Command::new("devconsole_cli")
        .version("1.0.0")
        .about("DevConsole CLI ツール")
        .arg(
            Arg::new("server")
                .short('s')
                .long("server")
                .value_name("ADDRESS")
                .help("DevConsole サーバーのアドレス")
                .default_value("ws://127.0.0.1:9001"),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Node ID を表示")
                .action(clap::ArgAction::SetTrue),
        )
        .subcommand(
            Command::new("listen")
                .about("指定したチャンネルを監視し、受信したメッセージを表示")
                .arg(
                    Arg::new("channel")
                        .help("チャンネル名またはID")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            Command::new("send")
                .about("指定したチャンネルにメッセージを送信")
                .arg(
                    Arg::new("binary")
                        .short('b')
                        .help("バイナリモードで送信")
                        .required(false)
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("channel")
                        .help("チャンネル名またはID")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("message")
                        .help("送信するメッセージ")
                        .required(true)
                        .index(2),
                ),
        )
        .subcommand(Command::new("list").about("利用可能なチャンネル一覧を表示"))
        .subcommand(
            Command::new("open")
                .about("指定した名前でチャンネルを開く")
                .arg(
                    Arg::new("name")
                        .help("チャンネル名")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            Command::new("info")
                .about("指定したチャンネルの情報を表示")
                .arg(
                    Arg::new("channel")
                        .help("チャンネル名またはID")
                        .required(true)
                        .index(1),
                ),
        )
        .get_matches();

    let server_addr = matches.get_one::<String>("server").unwrap();

    let mut client = match DCClient::new(server_addr).await {
        Ok(client) => client,
        Err(e) => {
            error!("サーバーへの接続に失敗しました: {e}");
            std::process::exit(1);
        }
    };

    // Wait a bit to receive NodeID notification
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    if matches.get_flag("verbose") {
        if let Some(node_id) = client.get_node_id().await {
            println!("Node ID: {node_id}");
        } else {
            println!("Node ID: 取得中...");
        }
    }

    match matches.subcommand() {
        Some(("listen", sub_matches)) => {
            if let Err(e) = handle_listen(&mut client, sub_matches).await {
                error!("Listen コマンドでエラーが発生しました: {e}");
                std::process::exit(1);
            }
        }
        Some(("send", sub_matches)) => {
            if let Err(e) = handle_send(&mut client, sub_matches).await {
                error!("Send コマンドでエラーが発生しました: {e}");
                std::process::exit(1);
            }
        }
        Some(("list", _)) => {
            if let Err(e) = handle_list(&mut client).await {
                error!("List コマンドでエラーが発生しました: {e}");
                std::process::exit(1);
            }
        }
        Some(("open", sub_matches)) => {
            if let Err(e) = handle_open(&mut client, sub_matches).await {
                error!("Open コマンドでエラーが発生しました: {e}");
                std::process::exit(1);
            }
        }
        Some(("info", sub_matches)) => {
            if let Err(e) = handle_info(&mut client, sub_matches).await {
                error!("Info コマンドでエラーが発生しました: {e}");
                std::process::exit(1);
            }
        }
        _ => {
            println!("コマンドを指定してください。--help でヘルプを表示します。");
        }
    }
}

async fn resolve_channel_id(
    client: &mut DCClient,
    channel_input: &str,
) -> Result<ChannelID, String> {
    // Try to parse as numeric ID first
    if let Ok(channel_id) = channel_input.parse::<ChannelID>() {
        return Ok(channel_id);
    }

    // If not numeric, search by name
    let channels = match client.channel_list().await {
        Ok(channels) => channels,
        Err(e) => return Err(format!("チャンネル一覧の取得に失敗しました: {e}")),
    };

    for &channel_id in &channels {
        match client.channel_info(channel_id).await {
            Ok(info) => {
                if info.name == channel_input {
                    return Ok(channel_id);
                }
            }
            Err(_) => continue, // Skip channels we can't get info for
        }
    }

    Err(format!("チャンネル '{channel_input}' が見つかりません"))
}

async fn handle_listen(client: &mut DCClient, matches: &ArgMatches) -> Result<(), String> {
    let channel_input = matches.get_one::<String>("channel").unwrap();
    let channel_id = resolve_channel_id(client, channel_input).await?;

    let (tx, mut rx) = mpsc::channel::<(ChannelID, String)>(64);
    let (tx_bin, mut rx_bin) = mpsc::channel::<(ChannelID, Vec<u8>)>(64);
    client
        .listen(channel_id, Some(tx), Some(tx_bin))
        .await
        .map_err(|e| format!("チャンネルの監視に失敗しました: {e}"))?;

    println!("チャンネル {channel_id} を監視しています。Ctrl+C で終了します。");
    loop {
        select! {
            Some((_, message)) = rx.recv() => {
                println!("{message}");
                io::stdout().flush().ok();
            }
            Some((_, data)) = rx_bin.recv() => {
                let mut s = String::new();
                for &b in &data {
                    match b {
                        b'\x1b' => s.push_str(r"\e"),
                        b'\n' => s.push_str(r"\n"),
                        b'\r' => s.push_str(r"\r"),
                        b'\t' => s.push_str(r"\t"),
                        b'\0' => s.push_str(r"\0"),
                        0x20..=0x7e => s.push(b as char),
                        _ => s.push_str(&format!(r"\x{b:02X}")),
                    }
                }

                println!("b'{s}'");
                io::stdout().flush().ok();
            }
            else => {
                // Both channels closed
                break;
            }
        }
    }

    Ok(())
}

fn intercept_escape_sequences(input: &str) -> Vec<u8> {
    let mut output = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '\\' {
            output.push(c as u8);
            continue;
        }
        let next_char = match chars.next() {
            Some(nc) => nc,
            None => {
                output.push(b'\\');
                break;
            }
        };
        match next_char {
            'n' => output.push(b'\n'),
            'r' => output.push(b'\r'),
            't' => output.push(b'\t'),
            'e' => output.push(b'\x1b'),
            '0' => output.push(b'\0'),
            'x' => {
                let hex1 = chars.next();
                let hex2 = chars.next();
                if let (Some(h1), Some(h2)) = (hex1, hex2) {
                    if let (Some(d1), Some(d2)) = (h1.to_digit(16), h2.to_digit(16)) {
                        output.push(((d1 << 4) | d2) as u8 );
                    } else {
                        output.push(b'\\');
                        output.push(b'x');
                        output.push(h1 as u8);
                        output.push(h2 as u8);
                    }
                }
            }
            _ => {
                output.push(b'\\');
                output.push(next_char as u8);
            }
        }
    }

    output
}

async fn handle_send(client: &mut DCClient, matches: &ArgMatches) -> Result<(), String> {
    let channel_input = matches.get_one::<String>("channel").unwrap();
    let message = matches.get_one::<String>("message").unwrap();
    let is_binary = matches.get_flag("binary");

    let channel_id = resolve_channel_id(client, channel_input).await?;

    if is_binary {
        client
            .send_bin(channel_id, intercept_escape_sequences(message))
            .await
            .map_err(|e| format!("メッセージの送信に失敗しました: {e}"))?;
    } else {
        client
            .send(channel_id, message.clone())
            .await
            .map_err(|e| format!("メッセージの送信に失敗しました: {e}"))?;
    }

    println!("チャンネル {channel_id} にメッセージを送信しました: {message}");

    Ok(())
}

async fn handle_list(client: &mut DCClient) -> Result<(), String> {
    let channels = client
        .channel_list()
        .await
        .map_err(|e| format!("チャンネル一覧の取得に失敗しました: {e}"))?;

    if channels.is_empty() {
        println!("利用可能なチャンネルはありません。");
        return Ok(());
    }

    println!("利用可能なチャンネル:");
    for &channel_id in &channels {
        match client.channel_info(channel_id).await {
            Ok(info) => {
                println!(
                    "  ID: {}, 名前: {}, 提供者: {}",
                    info.channel, info.name, info.supplied_by
                );
            }
            Err(_) => {
                println!("  ID: {channel_id} (情報取得エラー)");
            }
        }
    }

    Ok(())
}

async fn handle_open(client: &mut DCClient, matches: &ArgMatches) -> Result<(), String> {
    let name = matches.get_one::<String>("name").unwrap();

    let channel_id = client
        .open(name.clone())
        .await
        .map_err(|e| format!("チャンネルの作成に失敗しました: {e}"))?;

    println!("チャンネルを開きました - ID: {channel_id}, 名前: {name}");

    Ok(())
}

async fn handle_info(client: &mut DCClient, matches: &ArgMatches) -> Result<(), String> {
    let channel_input = matches.get_one::<String>("channel").unwrap();
    let channel_id = resolve_channel_id(client, channel_input).await?;

    let info = client
        .channel_info(channel_id)
        .await
        .map_err(|e| format!("チャンネル情報の取得に失敗しました: {e}"))?;

    println!("チャンネル情報:");
    println!("  ID: {}", info.channel);
    println!("  名前: {}", info.name);
    println!("  提供者: {}", info.supplied_by);

    Ok(())
}
