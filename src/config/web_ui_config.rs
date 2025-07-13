use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct WebUIConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_base_port")]
    pub base_port: u16,
    #[serde(default = "default_cols")]
    pub cols: u16,
    #[serde(default = "default_rows")]
    pub rows: u16,
}

impl Default for WebUIConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            host: default_host(),
            base_port: default_base_port(),
            cols: default_cols(),
            rows: default_rows(),
        }
    }
}

fn default_base_port() -> u16 {
    9990
}

fn default_enabled() -> bool {
    true
}

fn default_host() -> String {
    "localhost".to_string()
}

fn default_cols() -> u16 {
    80
}

fn default_rows() -> u16 {
    24
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_web_ui_config() {
        let config = WebUIConfig::default();
        assert_eq!(config.enabled, true);
        assert_eq!(config.host, "localhost");
        assert_eq!(config.base_port, 9990);
        assert_eq!(config.cols, 80);
        assert_eq!(config.rows, 24);
    }

    #[test]
    fn test_default_functions() {
        assert_eq!(default_base_port(), 9990);
        assert_eq!(default_enabled(), true);
        assert_eq!(default_host(), "localhost");
        assert_eq!(default_cols(), 80);
        assert_eq!(default_rows(), 24);
    }

    #[test]
    fn test_web_ui_config_deserialization() {
        let yaml = r#"
enabled: false
host: "0.0.0.0"
base_port: 8080
cols: 120
rows: 30
"#;
        let config: WebUIConfig = serde_yml::from_str(yaml).unwrap();
        assert_eq!(config.enabled, false);
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.base_port, 8080);
        assert_eq!(config.cols, 120);
        assert_eq!(config.rows, 30);
    }

    #[test]
    fn test_web_ui_config_partial_deserialization() {
        let yaml = r#"
base_port: 8000
"#;
        let config: WebUIConfig = serde_yml::from_str(yaml).unwrap();
        assert_eq!(config.enabled, true); // default
        assert_eq!(config.host, "localhost"); // default
        assert_eq!(config.base_port, 8000); // specified
        assert_eq!(config.cols, 80); // default
        assert_eq!(config.rows, 24); // default
    }
}
