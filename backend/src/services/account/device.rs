/// What the user agent `agent` says signed in, as a person reads it.
pub fn device_name(agent: Option<&str>) -> String {
    let Some(agent) = agent.map(str::trim).filter(|agent| !agent.is_empty()) else {
        return "Unknown device".to_owned();
    };
    match app_system(agent) {
        Some(system) => format!("Chess app on {}", system_name(system)),
        None => agent.to_owned(),
    }
}

/// The system in this app's own agent, `Chess <version> (<system>)`.
fn app_system(agent: &str) -> Option<&str> {
    let rest = agent.strip_prefix("Chess ")?;
    let (_, system) = rest.split_once(" (")?;
    system.strip_suffix(')')
}

/// Rust's name for a system, as its maker writes it.
fn system_name(system: &str) -> &str {
    match system {
        "macos" => "macOS",
        "windows" => "Windows",
        "linux" => "Linux",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::device_name;

    #[test]
    fn the_app_is_named_with_its_system_and_anything_else_as_it_came() {
        assert_eq!(
            device_name(Some("Chess 0.1.0 (macos)")),
            "Chess app on macOS"
        );
        assert_eq!(
            device_name(Some("Chess 0.1.0 (freebsd)")),
            "Chess app on freebsd"
        );
        assert_eq!(device_name(Some("curl/8.4.0")), "curl/8.4.0");
        assert_eq!(device_name(Some("  ")), "Unknown device");
        assert_eq!(device_name(None), "Unknown device");
    }
}
