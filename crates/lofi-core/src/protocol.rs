use crate::{Micros, NodeId};

pub const WIRE_LEN: usize = 40;
pub const MAGIC: [u8; 4] = *b"LOFI";
pub const VERSION: u8 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MessageKind {
    Sync = 1,
    Beat = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Frame {
    pub kind: MessageKind,
    pub node_id: NodeId,
    pub root_id: NodeId,
    pub sequence: u32,
    pub root_time_us: Micros,
    pub beat_period_us: u32,
    pub flags: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeError {
    Length,
    Magic,
    Version,
    Kind,
    Checksum,
}

impl Frame {
    pub fn encode(&self) -> [u8; WIRE_LEN] {
        let mut out = [0u8; WIRE_LEN];
        out[0..4].copy_from_slice(&MAGIC);
        out[4] = VERSION;
        out[5] = self.kind as u8;
        out[6..14].copy_from_slice(&self.node_id.to_le_bytes());
        out[14..22].copy_from_slice(&self.root_id.to_le_bytes());
        out[22..26].copy_from_slice(&self.sequence.to_le_bytes());
        out[26..34].copy_from_slice(&self.root_time_us.to_le_bytes());
        out[34..38].copy_from_slice(&self.beat_period_us.to_le_bytes());
        out[38..40].copy_from_slice(&self.flags.to_le_bytes());
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        if bytes.len() != WIRE_LEN {
            return Err(DecodeError::Length);
        }
        if bytes[0..4] != MAGIC {
            return Err(DecodeError::Magic);
        }
        if bytes[4] != VERSION {
            return Err(DecodeError::Version);
        }
        let kind = match bytes[5] {
            1 => MessageKind::Sync,
            2 => MessageKind::Beat,
            _ => return Err(DecodeError::Kind),
        };

        Ok(Self {
            kind,
            node_id: u64_from(&bytes[6..14]),
            root_id: u64_from(&bytes[14..22]),
            sequence: u32_from(&bytes[22..26]),
            root_time_us: i64_from(&bytes[26..34]),
            beat_period_us: u32_from(&bytes[34..38]),
            flags: u16_from(&bytes[38..40]),
        })
    }
}

fn u64_from(bytes: &[u8]) -> u64 {
    let mut arr = [0u8; 8];
    arr.copy_from_slice(bytes);
    u64::from_le_bytes(arr)
}

fn i64_from(bytes: &[u8]) -> i64 {
    let mut arr = [0u8; 8];
    arr.copy_from_slice(bytes);
    i64::from_le_bytes(arr)
}

fn u32_from(bytes: &[u8]) -> u32 {
    let mut arr = [0u8; 4];
    arr.copy_from_slice(bytes);
    u32::from_le_bytes(arr)
}

fn u16_from(bytes: &[u8]) -> u16 {
    let mut arr = [0u8; 2];
    arr.copy_from_slice(bytes);
    u16::from_le_bytes(arr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let frame = Frame {
            kind: MessageKind::Sync,
            node_id: 7,
            root_id: 1,
            sequence: 42,
            root_time_us: 123_456,
            beat_period_us: 500_000,
            flags: 0,
        };
        assert_eq!(Frame::decode(&frame.encode()), Ok(frame));
    }
}
