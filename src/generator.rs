//! Finite dual-tone signaling through the released tone component.

use crate::{DtmfDigit, tone_client::Sequence};

/// Invalid digit timing/level or an unavailable tone provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeneratorError;

/// Exclusive finite 48 kHz mono F32 tone followed by its interdigit silence.
pub struct DtmfGenerator(Sequence);

impl DtmfGenerator {
    /// Prepare off audio. The level is the combined peak of two equal components.
    pub fn new(
        digit: DtmfDigit,
        duration_ms: u32,
        gap_ms: u32,
        level_dbfs: i8,
    ) -> Result<Self, GeneratorError> {
        if !(1..=1000).contains(&duration_ms)
            || !(1..=1000).contains(&gap_ms)
            || !(-60..=0).contains(&level_dbfs)
        {
            return Err(GeneratorError);
        }
        let (row, column) = digit.frequencies();
        Sequence::new(
            &format!("{row}+{column}/{duration_ms},silence/{gap_ms}"),
            level_dbfs,
        )
        .map(Self)
    }

    /// Fill the leading output span without allocation; zero means complete or empty output.
    pub fn render(&mut self, output: &mut [f32]) -> Result<usize, GeneratorError> {
        self.0.render(output)
    }
}
