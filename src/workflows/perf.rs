use std::time::Duration;

pub fn perf_enabled() -> bool {
    match std::env::var("SYMM_PERF_LOG") {
        Ok(raw) => matches!(
            raw.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes"
        ),
        Err(_) => false,
    }
}

pub fn log_perf(event: &str, elapsed: Duration, fields: &[(&str, String)]) {
    if !perf_enabled() {
        return;
    }
    let mut parts = vec![
        format!("event={event}"),
        format!("elapsed_ms={}", elapsed.as_millis()),
    ];
    for (key, value) in fields {
        parts.push(format!("{key}={value}"));
    }
    eprintln!("[symm-perf] {}", parts.join(" "));
}
