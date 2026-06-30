/// Tiny deterministic LCG used for drift, latency, and packet-loss jitter.
#[derive(Debug)]
pub struct Lcg {
    state: u64,
}

impl Lcg {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }

    pub fn next_bounded(&mut self, upper: u64) -> u64 {
        self.next() % upper.max(1)
    }

    pub fn range_i32(&mut self, min: i32, max: i32) -> i32 {
        min + self.next_bounded((max - min) as u64) as i32
    }

    pub fn range_i64(&mut self, min: i64, max: i64) -> i64 {
        min + self.next_bounded((max - min) as u64) as i64
    }
}
