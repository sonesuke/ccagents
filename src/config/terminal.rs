/// Terminal configuration containing dimensions
#[derive(Debug, Clone)]
pub struct TerminalConfig {
    pub cols: u16,
    pub rows: u16,
}

impl TerminalConfig {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self { cols, rows }
    }

    /// Get dimensions as a tuple for compatibility
    #[allow(dead_code)]
    pub fn dimensions(&self) -> (u16, u16) {
        (self.cols, self.rows)
    }
}
