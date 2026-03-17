#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleFormat {
    Float64,
    Float32,
    Signed32,
    Unsigned32,
    Signed24,
    Unsigned24,
    Signed16,
    Unsigned16,
    Signed8,
    Unsigned8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelSpec {
    Count(u16),
}

impl ChannelSpec {
    pub fn count(self) -> u16 {
        match self {
            ChannelSpec::Count(count) => count,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferSize {
    /// Inclusive range of supported buffer sizes.
    Range(u32, u32),
    Fixed(u32),
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatInfo {
    pub originating_provider: &'static str,
    pub sample_type: SampleFormat,
    pub sample_rate: u32,
    pub buffer_size: BufferSize,
    pub channels: ChannelSpec,
}

/// TODO: this will be used in the future
#[allow(dead_code)]
pub struct SupportedFormat {
    pub originating_provider: &'static str,
    pub sample_type: SampleFormat,
    /// Lowest and highest supported sample rates.
    pub sample_rates: (u32, u32),
    pub buffer_size: BufferSize,
    pub channels: ChannelSpec,
}
