//! Compact, ESP-NOW-ready wire format for the mesh sync protocol.
//!
//! Three message kinds, all well under the ESP-NOW v1 250-byte payload limit:
//! a periodic broadcast `Beacon` (topology + coarse time) and the two halves of
//! an NTP-style timing exchange, `ProbeRequest` / `ProbeResponse`. All
//! timestamps are local monotonic microseconds; the firmware stamps receive
//! times as close to the radio interrupt as it can and transmit times at send.

use crate::{Micros, NodeId};

pub const MESH_VERSION: u8 = 1;
pub const MESH_WIRE_MAX: usize = 72;

const KIND_BEACON: u8 = 1;
const KIND_PROBE_REQUEST: u8 = 2;
const KIND_PROBE_RESPONSE: u8 = 3;

/// A periodic broadcast: who I am, who I think the mesh root is, how far I am
/// from it, and my current mesh-time estimate. Cheap topology + coarse sync.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Beacon {
    pub sender: NodeId,
    pub root_id: NodeId,
    pub epoch: u32,
    pub stratum: u8,
    pub seq: u32,
    pub mesh_us: Micros,
    pub rate_ppb: i32,
    pub root_dispersion_us: i32,
}

/// First half of a timing exchange. `t1` is the sender's local send time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProbeRequest {
    pub sender: NodeId,
    pub target: NodeId,
    pub t1_local_us: Micros,
    pub seq: u32,
}

/// Second half. Carries the four-timestamp set plus the responder's mesh-time
/// reading at its transmit instant, so the requester can recover both one-way
/// delay and the reference time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProbeResponse {
    pub sender: NodeId,
    pub target: NodeId,
    pub t1_local_us: Micros,
    pub t2_local_us: Micros,
    pub t3_local_us: Micros,
    pub sender_mesh_us: Micros,
    pub root_id: NodeId,
    pub stratum: u8,
    pub seq: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshMessage {
    Beacon(Beacon),
    ProbeRequest(ProbeRequest),
    ProbeResponse(ProbeResponse),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WireError {
    Short,
    Version,
    Kind,
}

impl MeshMessage {
    /// Encode into a fixed buffer, returning the used length.
    pub fn encode(&self) -> ([u8; MESH_WIRE_MAX], usize) {
        let mut buf = [0u8; MESH_WIRE_MAX];
        let mut w = Writer::new(&mut buf);
        w.u8(MESH_VERSION);
        match self {
            MeshMessage::Beacon(b) => {
                w.u8(KIND_BEACON);
                w.u64(b.sender);
                w.u64(b.root_id);
                w.u32(b.epoch);
                w.u8(b.stratum);
                w.u32(b.seq);
                w.i64(b.mesh_us);
                w.i32(b.rate_ppb);
                w.i32(b.root_dispersion_us);
            }
            MeshMessage::ProbeRequest(p) => {
                w.u8(KIND_PROBE_REQUEST);
                w.u64(p.sender);
                w.u64(p.target);
                w.i64(p.t1_local_us);
                w.u32(p.seq);
            }
            MeshMessage::ProbeResponse(p) => {
                w.u8(KIND_PROBE_RESPONSE);
                w.u64(p.sender);
                w.u64(p.target);
                w.i64(p.t1_local_us);
                w.i64(p.t2_local_us);
                w.i64(p.t3_local_us);
                w.i64(p.sender_mesh_us);
                w.u64(p.root_id);
                w.u8(p.stratum);
                w.u32(p.seq);
            }
        }
        let len = w.pos;
        (buf, len)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        let mut r = Reader::new(bytes);
        if r.u8()? != MESH_VERSION {
            return Err(WireError::Version);
        }
        match r.u8()? {
            KIND_BEACON => Ok(MeshMessage::Beacon(Beacon {
                sender: r.u64()?,
                root_id: r.u64()?,
                epoch: r.u32()?,
                stratum: r.u8()?,
                seq: r.u32()?,
                mesh_us: r.i64()?,
                rate_ppb: r.i32()?,
                root_dispersion_us: r.i32()?,
            })),
            KIND_PROBE_REQUEST => Ok(MeshMessage::ProbeRequest(ProbeRequest {
                sender: r.u64()?,
                target: r.u64()?,
                t1_local_us: r.i64()?,
                seq: r.u32()?,
            })),
            KIND_PROBE_RESPONSE => Ok(MeshMessage::ProbeResponse(ProbeResponse {
                sender: r.u64()?,
                target: r.u64()?,
                t1_local_us: r.i64()?,
                t2_local_us: r.i64()?,
                t3_local_us: r.i64()?,
                sender_mesh_us: r.i64()?,
                root_id: r.u64()?,
                stratum: r.u8()?,
                seq: r.u32()?,
            })),
            _ => Err(WireError::Kind),
        }
    }
}

struct Writer<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> Writer<'a> {
    fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }
    fn put(&mut self, bytes: &[u8]) {
        self.buf[self.pos..self.pos + bytes.len()].copy_from_slice(bytes);
        self.pos += bytes.len();
    }
    fn u8(&mut self, v: u8) {
        self.put(&[v]);
    }
    fn u32(&mut self, v: u32) {
        self.put(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.put(&v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.put(&v.to_le_bytes());
    }
    fn i64(&mut self, v: i64) {
        self.put(&v.to_le_bytes());
    }
}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }
    fn take<const N: usize>(&mut self) -> Result<[u8; N], WireError> {
        if self.pos + N > self.buf.len() {
            return Err(WireError::Short);
        }
        let mut out = [0u8; N];
        out.copy_from_slice(&self.buf[self.pos..self.pos + N]);
        self.pos += N;
        Ok(out)
    }
    fn u8(&mut self) -> Result<u8, WireError> {
        Ok(self.take::<1>()?[0])
    }
    fn u32(&mut self) -> Result<u32, WireError> {
        Ok(u32::from_le_bytes(self.take::<4>()?))
    }
    fn u64(&mut self) -> Result<u64, WireError> {
        Ok(u64::from_le_bytes(self.take::<8>()?))
    }
    fn i32(&mut self) -> Result<i32, WireError> {
        Ok(i32::from_le_bytes(self.take::<4>()?))
    }
    fn i64(&mut self) -> Result<i64, WireError> {
        Ok(i64::from_le_bytes(self.take::<8>()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(msg: MeshMessage) {
        let (buf, len) = msg.encode();
        assert!(len <= MESH_WIRE_MAX);
        assert_eq!(MeshMessage::decode(&buf[..len]), Ok(msg));
    }

    #[test]
    fn beacon_round_trips() {
        round_trip(MeshMessage::Beacon(Beacon {
            sender: 7,
            root_id: 1,
            epoch: 42,
            stratum: 2,
            seq: 99,
            mesh_us: 123_456_789,
            rate_ppb: -54_321,
            root_dispersion_us: 800,
        }));
    }

    #[test]
    fn probes_round_trip() {
        round_trip(MeshMessage::ProbeRequest(ProbeRequest {
            sender: 3,
            target: 1,
            t1_local_us: 9_000_000,
            seq: 5,
        }));
        round_trip(MeshMessage::ProbeResponse(ProbeResponse {
            sender: 1,
            target: 3,
            t1_local_us: 9_000_000,
            t2_local_us: 9_001_200,
            t3_local_us: 9_001_300,
            sender_mesh_us: 12_345_678,
            root_id: 1,
            stratum: 0,
            seq: 5,
        }));
    }

    #[test]
    fn rejects_short_and_bad_version() {
        assert_eq!(MeshMessage::decode(&[]), Err(WireError::Short));
        assert_eq!(MeshMessage::decode(&[2, 1]), Err(WireError::Version));
        assert_eq!(
            MeshMessage::decode(&[MESH_VERSION, 9]),
            Err(WireError::Kind)
        );
    }
}
