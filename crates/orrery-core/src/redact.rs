use std::path::Path;

pub const SECRET: &str = "***";
pub const USER: &str = "<user>";
pub const ADDRESS: &str = "<ip>";
const MIN_SECRET_LEN: usize = 4;
const MIN_NAME_LEN: usize = 3;
const TOKEN_FLAGS: [&str; 2] = ["--accessToken", "--session"];
const TOKEN_KEYS: [&str; 3] = [
    "\"accessToken\":\"",
    "\"authToken\":\"",
    "\"refreshToken\":\"",
];

#[derive(Debug, Clone, Default)]
pub struct Redactor {
    secrets: Vec<String>,
    homes: Vec<String>,
    names: Vec<String>,
}

impl Redactor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn system() -> Self {
        let mut redactor = Self::new();
        for key in ["USERPROFILE", "HOME"] {
            if let Some(home) = std::env::var_os(key) {
                redactor = redactor.home(Path::new(&home));
            }
        }
        for key in ["USERNAME", "USER", "LOGNAME"] {
            if let Ok(name) = std::env::var(key) {
                redactor = redactor.name(&name);
            }
        }
        redactor
    }

    pub fn secret(mut self, secret: &str) -> Self {
        if secret.len() >= MIN_SECRET_LEN && !self.secrets.iter().any(|s| s == secret) {
            self.secrets.push(secret.to_string());
        }
        self
    }

    pub fn home(mut self, home: &Path) -> Self {
        let home = home.to_string_lossy();
        let home = home.trim_end_matches(['/', '\\']);
        if home.len() < MIN_NAME_LEN {
            return self;
        }
        for variant in [
            home.to_string(),
            home.replace('\\', "/"),
            home.replace('/', "\\"),
            home.replace(['/', '\\'], "\\\\"),
        ] {
            if !self.homes.contains(&variant) {
                self.homes.push(variant);
            }
        }
        self.homes.sort_by_key(|home| std::cmp::Reverse(home.len()));
        self
    }

    pub fn name(mut self, name: &str) -> Self {
        let name = name.trim();
        if name.chars().count() >= MIN_NAME_LEN
            && !self.names.iter().any(|n| n.eq_ignore_ascii_case(name))
        {
            self.names.push(name.to_string());
            self.names.sort_by_key(|name| std::cmp::Reverse(name.len()));
        }
        self
    }

    pub fn redact(&self, text: &str) -> String {
        let mut text = text.to_string();
        for secret in &self.secrets {
            text = text.replace(secret, SECRET);
        }
        text = redact_jwts(&text);
        for flag in TOKEN_FLAGS {
            text = redact_after(&text, &format!("{flag} "), |c| c.is_whitespace());
        }
        for key in TOKEN_KEYS {
            text = redact_after(&text, key, |c| c == '"');
        }
        for home in &self.homes {
            text = replace_ignore_case(&text, home, "~", false);
        }
        for name in &self.names {
            text = replace_ignore_case(&text, name, USER, true);
        }
        redact_ipv6(&redact_ipv4(&text))
    }
}

pub fn redact_args(args: &[String], secret: &str) -> Vec<String> {
    let redactor = Redactor::new().secret(secret);
    args.iter().map(|arg| redactor.redact(arg)).collect()
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn replace_ignore_case(text: &str, needle: &str, replacement: &str, whole_word: bool) -> String {
    let lower_text = text.to_lowercase();
    let lower_needle = needle.to_lowercase();
    if lower_text.len() != text.len() || lower_needle.len() != needle.len() {
        return text.replace(needle, replacement);
    }
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    let mut search = 0;
    while let Some(found) = lower_text[search..].find(&lower_needle) {
        let start = search + found;
        let end = start + needle.len();
        let bounded = !whole_word
            || (!text[..start].chars().next_back().is_some_and(is_word_char)
                && !text[end..].chars().next().is_some_and(is_word_char));
        if bounded {
            out.push_str(&text[last..start]);
            out.push_str(replacement);
            last = end;
            search = end;
        } else {
            search = start + text[start..].chars().next().map_or(1, char::len_utf8);
        }
    }
    out.push_str(&text[last..]);
    out
}

fn redact_after(text: &str, marker: &str, stop: impl Fn(char) -> bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(found) = rest.find(marker) {
        let value_start = found + marker.len();
        out.push_str(&rest[..value_start]);
        let value = &rest[value_start..];
        let end = value.find(&stop).unwrap_or(value.len());
        if end > 0 {
            out.push_str(SECRET);
        }
        rest = &value[end..];
    }
    out.push_str(rest);
    out
}

fn redact_jwts(text: &str) -> String {
    let is_token_char = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.');
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(found) = rest.find("eyJ") {
        let token = &rest[found..];
        let end = token.find(|c| !is_token_char(c)).unwrap_or(token.len());
        let candidate = &token[..end];
        out.push_str(&rest[..found]);
        if candidate.matches('.').count() >= 2 && candidate.len() >= 20 {
            out.push_str(SECRET);
        } else {
            out.push_str(candidate);
        }
        rest = &token[end..];
    }
    out.push_str(rest);
    out
}

fn redact_ipv4(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    let mut i = 0;
    while i < bytes.len() {
        let starts_here = bytes[i].is_ascii_digit()
            && (i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'.'));
        if starts_here && let Some((end, octets)) = parse_ipv4(bytes, i) {
            if !matches!(octets, [127, ..] | [0, 0, 0, 0]) {
                out.push_str(&text[last..i]);
                out.push_str(ADDRESS);
                last = end;
            }
            i = end;
            continue;
        }
        i += 1;
    }
    out.push_str(&text[last..]);
    out
}

fn parse_ipv4(bytes: &[u8], start: usize) -> Option<(usize, [u8; 4])> {
    let mut octets = [0u8; 4];
    let mut pos = start;
    for (index, octet) in octets.iter_mut().enumerate() {
        if index > 0 {
            if bytes.get(pos) != Some(&b'.') {
                return None;
            }
            pos += 1;
        }
        let digits = bytes[pos..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count();
        if digits == 0 || digits > 3 {
            return None;
        }
        *octet = std::str::from_utf8(&bytes[pos..pos + digits])
            .ok()?
            .parse()
            .ok()?;
        pos += digits;
    }
    let continues = match bytes.get(pos) {
        Some(b) if b.is_ascii_alphanumeric() => true,
        Some(b'.') => bytes.get(pos + 1).is_some_and(u8::is_ascii_digit),
        _ => false,
    };
    (!continues).then_some((pos, octets))
}

fn redact_ipv6(text: &str) -> String {
    let is_part = |c: char| c.is_ascii_hexdigit() || c == ':';
    let mut out = String::with_capacity(text.len());
    let mut chars = text.char_indices().peekable();
    let mut last = 0;
    let mut prev: Option<char> = None;
    while let Some((start, c)) = chars.next() {
        if !is_part(c) || prev.is_some_and(is_word_char) {
            prev = Some(c);
            continue;
        }
        let mut end = start + c.len_utf8();
        while let Some(&(index, next)) = chars.peek() {
            if !is_part(next) {
                break;
            }
            end = index + next.len_utf8();
            chars.next();
        }
        let candidate = &text[start..end];
        let followed_by_word = text[end..].chars().next().is_some_and(is_word_char);
        if !followed_by_word && looks_like_ipv6(candidate) {
            out.push_str(&text[last..start]);
            out.push_str(ADDRESS);
            last = end;
        }
        prev = candidate.chars().next_back();
    }
    out.push_str(&text[last..]);
    out
}

fn looks_like_ipv6(candidate: &str) -> bool {
    let colons = candidate.matches(':').count();
    let groups: Vec<&str> = candidate.split(':').collect();
    if colons < 2 || groups.iter().any(|g| g.len() > 4) {
        return false;
    }
    let filled = groups.iter().filter(|g| !g.is_empty()).count();
    if candidate.contains("::") {
        filled >= 2 && candidate.matches("::").count() == 1
    } else {
        colons == 7 && filled == 8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_known_secrets_and_token_patterns() {
        let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0In0.c2lnbmF0dXJlLXZhbHVl";
        let text = format!(
            "--accessToken abc123secret --uuid 1\nSession ID is token:{jwt}:uuid\n{{\"accessToken\":\"xyz987\"}}\nsecret-value"
        );
        let redacted = Redactor::new().secret("secret-value").redact(&text);
        assert!(!redacted.contains("abc123secret"));
        assert!(!redacted.contains(jwt));
        assert!(!redacted.contains("xyz987"));
        assert!(!redacted.contains("secret-value"));
        assert!(redacted.contains("--accessToken *** --uuid 1"));
        assert!(redacted.contains("token:***:uuid"));
    }

    #[test]
    fn redacts_addresses_but_keeps_versions_and_times() {
        let text = "[12:34:56] Connecting to 203.0.113.7, 25565 via 2001:db8::8a2e:370:7334 and fe80:0:0:0:200:5efe:192:1\nLoaded 1.20.1 at 127.0.0.1 and ::1, Foo::bar, 0.0.0.0, java 21.0.5, mod 0.16.10";
        let redacted = Redactor::new().redact(text);
        assert_eq!(
            redacted,
            "[12:34:56] Connecting to <ip>, 25565 via <ip> and <ip>\nLoaded 1.20.1 at 127.0.0.1 and ::1, Foo::bar, 0.0.0.0, java 21.0.5, mod 0.16.10"
        );
    }

    #[test]
    fn redacts_home_paths_and_names() {
        let redactor = Redactor::new()
            .home(Path::new(r"C:\Users\Michel"))
            .name("Michel")
            .name("Steve_99")
            .name("ab");
        let text = r#"Loading C:\Users\Michel\AppData\x and c:/users/michel/.minecraft
args=["C:\\Users\\Michel\\AppData\\y"]
Setting user: Steve_99, michel joined, Michelle stays, ab stays"#;
        assert_eq!(
            redactor.redact(text),
            r#"Loading ~\AppData\x and ~/.minecraft
args=["~\\AppData\\y"]
Setting user: <user>, <user> joined, Michelle stays, ab stays"#
        );
    }

    #[test]
    fn redact_args_replaces_the_token() {
        let args = vec!["--accessToken".to_string(), "secret-token".to_string()];
        assert_eq!(redact_args(&args, "secret-token"), ["--accessToken", "***"]);
        assert_eq!(redact_args(&args, "0"), args);
    }
}
