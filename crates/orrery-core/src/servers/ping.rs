use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use ts_rs::TS;

use super::ServerAddress;
use crate::chat;
use crate::{CoreError, CoreResult};

const TIMEOUT: Duration = Duration::from_secs(6);
const STATUS_PROTOCOL: i32 = -1;
const MAX_PACKET_BYTES: usize = 2 * 1024 * 1024;
const MAX_MOTD_CHARS: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ServerStatus {
    pub version: Option<String>,
    pub protocol: Option<i64>,
    pub players_online: Option<i64>,
    pub players_max: Option<i64>,
    pub motd: String,
    pub favicon: Option<String>,
    pub latency_ms: u64,
}

pub async fn ping(address: &ServerAddress) -> CoreResult<ServerStatus> {
    tokio::time::timeout(TIMEOUT, status(address))
        .await
        .map_err(|_| unreachable(address, "no answer in time"))?
}

fn unreachable(address: &ServerAddress, reason: impl std::fmt::Display) -> CoreError {
    CoreError::Unreachable(format!("server {address} is not reachable: {reason}"))
}

async fn status(address: &ServerAddress) -> CoreResult<ServerStatus> {
    let started = Instant::now();
    let mut stream = TcpStream::connect((address.host.as_str(), address.port))
        .await
        .map_err(|err| unreachable(address, err))?;
    let connect_ms = started.elapsed().as_millis() as u64;
    stream.set_nodelay(true).ok();

    let mut handshake = Vec::new();
    write_varint(&mut handshake, 0);
    write_varint(&mut handshake, STATUS_PROTOCOL);
    write_string(&mut handshake, &address.host);
    handshake.extend(address.port.to_be_bytes());
    write_varint(&mut handshake, 1);
    send(&mut stream, &handshake, address).await?;
    send(&mut stream, &[0], address).await?;

    let packet = read_packet(&mut stream)
        .await
        .map_err(|err| unreachable(address, err))?;
    let mut body = packet.as_slice();
    if read_varint_slice(&mut body) != Some(0) {
        return Err(unreachable(address, "unexpected status packet"));
    }
    let json_len = read_varint_slice(&mut body)
        .and_then(|len| usize::try_from(len).ok())
        .filter(|len| *len <= body.len())
        .ok_or_else(|| unreachable(address, "malformed status packet"))?;
    let json: Value = serde_json::from_slice(&body[..json_len])
        .map_err(|err| unreachable(address, format!("invalid status: {err}")))?;

    let ping_started = Instant::now();
    let mut ping = Vec::new();
    write_varint(&mut ping, 1);
    ping.extend(0x4F52_5245_5259_i64.to_be_bytes());
    let latency_ms = match send(&mut stream, &ping, address).await {
        Ok(()) => match read_packet(&mut stream).await {
            Ok(_) => ping_started.elapsed().as_millis() as u64,
            Err(_) => connect_ms,
        },
        Err(_) => connect_ms,
    };
    Ok(parse_status(&json, latency_ms))
}

async fn send(stream: &mut TcpStream, payload: &[u8], address: &ServerAddress) -> CoreResult<()> {
    let mut packet = Vec::with_capacity(payload.len() + 5);
    write_varint(&mut packet, payload.len() as i32);
    packet.extend(payload);
    stream
        .write_all(&packet)
        .await
        .map_err(|err| unreachable(address, err))
}

fn parse_status(json: &Value, latency_ms: u64) -> ServerStatus {
    let number = |value: &Value| value.as_i64();
    ServerStatus {
        version: json["version"]["name"]
            .as_str()
            .map(|name| chat::strip_formatting(name).chars().take(64).collect()),
        protocol: number(&json["version"]["protocol"]),
        players_online: number(&json["players"]["online"]),
        players_max: number(&json["players"]["max"]),
        motd: chat::plain_text(&json["description"])
            .lines()
            .map(str::trim)
            .collect::<Vec<_>>()
            .join("\n")
            .chars()
            .take(MAX_MOTD_CHARS)
            .collect(),
        favicon: json["favicon"]
            .as_str()
            .filter(|icon| icon.starts_with("data:image/png;base64,") && icon.len() < 512 * 1024)
            .map(str::to_string),
        latency_ms,
    }
}

fn write_varint(out: &mut Vec<u8>, value: i32) {
    let mut value = value as u32;
    loop {
        if value & !0x7F == 0 {
            out.push(value as u8);
            return;
        }
        out.push((value as u8 & 0x7F) | 0x80);
        value >>= 7;
    }
}

fn write_string(out: &mut Vec<u8>, value: &str) {
    write_varint(out, value.len() as i32);
    out.extend(value.as_bytes());
}

fn read_varint_slice(data: &mut &[u8]) -> Option<i32> {
    let mut value = 0u32;
    for shift in (0..35).step_by(7) {
        let (&byte, rest) = data.split_first()?;
        *data = rest;
        value |= u32::from(byte & 0x7F) << shift;
        if byte & 0x80 == 0 {
            return Some(value as i32);
        }
    }
    None
}

async fn read_varint(reader: &mut (impl AsyncRead + Unpin)) -> std::io::Result<i32> {
    let mut value = 0u32;
    for shift in (0..35).step_by(7) {
        let byte = reader.read_u8().await?;
        value |= u32::from(byte & 0x7F) << shift;
        if byte & 0x80 == 0 {
            return Ok(value as i32);
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "varint is too long",
    ))
}

async fn read_packet(reader: &mut (impl AsyncRead + Unpin)) -> std::io::Result<Vec<u8>> {
    let len = usize::try_from(read_varint(reader).await?)
        .ok()
        .filter(|len| *len > 0 && *len <= MAX_PACKET_BYTES)
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid packet length")
        })?;
    let mut packet = vec![0; len];
    reader.read_exact(&mut packet).await?;
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    async fn fake_server(status: &'static str) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let handshake = read_packet(&mut socket).await.unwrap();
            let mut body = handshake.as_slice();
            assert_eq!(read_varint_slice(&mut body), Some(0));
            assert_eq!(read_varint_slice(&mut body), Some(-1));
            let request = read_packet(&mut socket).await.unwrap();
            assert_eq!(request, [0]);
            let mut response = Vec::new();
            write_varint(&mut response, 0);
            write_string(&mut response, status);
            let mut framed = Vec::new();
            write_varint(&mut framed, response.len() as i32);
            framed.extend(response);
            socket.write_all(&framed).await.unwrap();
            let ping = read_packet(&mut socket).await.unwrap();
            let mut framed = Vec::new();
            write_varint(&mut framed, ping.len() as i32);
            framed.extend(ping);
            socket.write_all(&framed).await.unwrap();
        });
        port
    }

    #[tokio::test]
    async fn reads_a_modern_status() {
        let port = fake_server(
            r#"{"version":{"name":"Paper 1.21.1","protocol":767},"players":{"max":100,"online":7},"description":{"text":"§aWelcome ","extra":[{"text":"to "},"Orrery\n  second line"]},"favicon":"data:image/png;base64,iVBORw0KGgo="}"#,
        )
        .await;
        let status = ping(&ServerAddress {
            host: "127.0.0.1".into(),
            port,
        })
        .await
        .unwrap();
        assert_eq!(status.version.as_deref(), Some("Paper 1.21.1"));
        assert_eq!(status.protocol, Some(767));
        assert_eq!(
            (status.players_online, status.players_max),
            (Some(7), Some(100))
        );
        assert_eq!(status.motd, "Welcome to Orrery\nsecond line");
        assert_eq!(
            status.favicon.as_deref(),
            Some("data:image/png;base64,iVBORw0KGgo=")
        );
    }

    #[tokio::test]
    async fn reports_closed_ports() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let err = ping(&ServerAddress {
            host: "127.0.0.1".into(),
            port,
        })
        .await
        .unwrap_err();
        assert_eq!(err.kind(), "unreachable");
    }

    #[test]
    fn varints_roundtrip() {
        for value in [0, 1, 127, 128, 255, 25565, 2_097_151, -1, i32::MAX] {
            let mut out = Vec::new();
            write_varint(&mut out, value);
            let mut slice = out.as_slice();
            assert_eq!(read_varint_slice(&mut slice), Some(value));
            assert!(slice.is_empty());
        }
        assert_eq!(read_varint_slice(&mut [0xFF; 6].as_slice()), None);
    }

    #[test]
    fn parses_plain_string_descriptions() {
        let status = parse_status(
            &serde_json::json!({"description": "§cA §lMinecraft Server", "players": {"online": 0}}),
            12,
        );
        assert_eq!(status.motd, "A Minecraft Server");
        assert_eq!(status.players_max, None);
        assert_eq!(status.version, None);
        assert_eq!(status.latency_ms, 12);
    }
}
