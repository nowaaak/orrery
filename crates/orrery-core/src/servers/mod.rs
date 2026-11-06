pub mod list;
pub mod ping;
pub mod playtime;

use std::fmt;

use crate::services::Services;
use crate::{CoreError, CoreResult};

pub const DEFAULT_PORT: u16 = 25565;
const MAX_HOST_LEN: usize = 253;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerAddress {
    pub host: String,
    pub port: u16,
}

impl ServerAddress {
    pub fn parse(input: &str) -> CoreResult<Self> {
        let input = input.trim();
        let invalid = || CoreError::InvalidInput(format!("invalid server address {input:?}"));
        let (host, port) = if let Some(rest) = input.strip_prefix('[') {
            let (host, after) = rest.split_once(']').ok_or_else(invalid)?;
            if !host.contains(':') || !host.chars().all(|c| c.is_ascii_hexdigit() || c == ':') {
                return Err(invalid());
            }
            match after {
                "" => (host, None),
                port => (host, Some(port.strip_prefix(':').ok_or_else(invalid)?)),
            }
        } else {
            match input.split_once(':') {
                Some((host, port)) => (host, Some(port)),
                None => (input, None),
            }
        };
        let valid_host = !host.is_empty()
            && host.len() <= MAX_HOST_LEN
            && !host.starts_with(['-', '.'])
            && host
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':'));
        if !valid_host {
            return Err(invalid());
        }
        let port = match port {
            None => DEFAULT_PORT,
            Some(port) => port
                .parse::<u16>()
                .ok()
                .filter(|port| *port != 0)
                .ok_or_else(invalid)?,
        };
        Ok(Self {
            host: host.to_string(),
            port,
        })
    }
}

impl Services {
    pub async fn ping_server(&self, address: &str) -> CoreResult<ping::ServerStatus> {
        ping::ping(&ServerAddress::parse(address)?).await
    }
}

impl fmt::Display for ServerAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.host.contains(':') {
            write!(f, "[{}]:{}", self.host, self.port)
        } else {
            write!(f, "{}:{}", self.host, self.port)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hosts_ports_and_ipv6() {
        assert_eq!(
            ServerAddress::parse(" play.example.org ").unwrap(),
            ServerAddress {
                host: "play.example.org".into(),
                port: 25565
            }
        );
        let local = ServerAddress::parse("localhost:25566").unwrap();
        assert_eq!(local.port, 25566);
        assert_eq!(local.to_string(), "localhost:25566");
        let v6 = ServerAddress::parse("[::1]:25570").unwrap();
        assert_eq!(v6.host, "::1");
        assert_eq!(v6.to_string(), "[::1]:25570");
        assert_eq!(ServerAddress::parse("[::1]").unwrap().port, 25565);
    }

    #[test]
    fn rejects_invalid_addresses() {
        for input in [
            "",
            ":25565",
            "host:0",
            "host:70000",
            "host:abc",
            "bad host",
            "-flag",
            "host;rm",
            "[::1",
            "[zz]:1",
            "[::1]25565",
        ] {
            assert!(ServerAddress::parse(input).is_err(), "{input}");
        }
    }
}
