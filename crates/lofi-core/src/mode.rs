use crate::event::Section;
use crate::groove::{sample_i16, GrooveConfig, GroovePart};
use crate::transport::Transport;
use crate::Micros;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GrooveContext {
    pub root_time_us: Micros,
    pub transport: Transport,
    pub section: Section,
    pub sample_rate: u32,
    pub seed: u64,
    pub density_milli: u16,
    pub swing_milli: u16,
    pub variation: u32,
}

pub trait GrooveModeEngine {
    fn id(&self) -> GrooveModeId;
    fn sample_i16(&self, ctx: GrooveContext, part: GroovePart) -> i16;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GrooveModeId {
    DustyTape,
    JazzHop,
    AmbientStudy,
    DrumOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DustyTape;

impl GrooveModeEngine for DustyTape {
    fn id(&self) -> GrooveModeId {
        GrooveModeId::DustyTape
    }

    fn sample_i16(&self, ctx: GrooveContext, part: GroovePart) -> i16 {
        sample_i16(
            ctx.root_time_us,
            ctx.transport,
            ctx.section,
            GrooveConfig {
                sample_rate: ctx.sample_rate,
                seed: ctx.seed ^ ctx.variation as u64,
                part,
            },
        )
    }
}
