const SENSITIVE_MARKERS: &[&str] = &[
    "api_key",
    "apikey",
    "authorization",
    "cookie",
    "key",
    "openai_api_key",
    "password",
    "refresh_token",
    "secret",
    "session",
    "sk-",
    "token",
];

pub fn is_sensitive_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    SENSITIVE_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
}

pub fn redact_named_value(name: &str, value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }

    if is_sensitive_name(name) {
        redact_secret(value)
    } else {
        value.to_string()
    }
}

pub fn redact_secret(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    match chars.len() {
        0 => String::new(),
        1..=6 => "***".to_string(),
        len => {
            let prefix: String = chars.iter().take(3).collect();
            let suffix: String = chars.iter().skip(len.saturating_sub(3)).collect();
            format!("{prefix}***{suffix}")
        }
    }
}
