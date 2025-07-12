/// Terminal configuration containing dimensions and shell settings
#[derive(Debug, Clone)]
pub struct TerminalConfig {
    pub cols: u16,
    pub rows: u16,
    pub shell_command: String,
}

impl TerminalConfig {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols,
            rows,
            shell_command: std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string()),
        }
    }

    #[allow(dead_code)]
    pub fn with_shell(cols: u16, rows: u16, shell_command: String) -> Self {
        Self {
            cols,
            rows,
            shell_command,
        }
    }

    /// Get dimensions as a tuple for compatibility
    #[allow(dead_code)]
    pub fn dimensions(&self) -> (u16, u16) {
        (self.cols, self.rows)
    }
}
