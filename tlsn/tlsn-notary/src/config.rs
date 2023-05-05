const DEFAULT_MAX_TRANSCRIPT_SIZE: usize = 2 << 14; // 16Kb

#[derive(Debug, Clone, derive_builder::Builder)]
pub struct NotaryConfig {
    /// Maximum transcript size in bytes
    ///
    /// This includes the number of bytes sent and received to the server.
    #[builder(default = "DEFAULT_MAX_TRANSCRIPT_SIZE")]
    max_transcript_size: usize,
}

impl NotaryConfig {
    /// Create a new builder for `NotaryConfig`.
    pub fn builder() -> NotaryConfigBuilder {
        NotaryConfigBuilder::default()
    }

    /// Get the maximum transcript size in bytes.
    pub fn max_transcript_size(&self) -> usize {
        self.max_transcript_size
    }
}