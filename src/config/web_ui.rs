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
