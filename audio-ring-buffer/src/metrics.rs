
use std::time::{Duration, Instant};

pub struct LatencyClock {
    origin: Instant,
}

impl Default for LatencyClock {
    fn default() -> Self {
        Self::new()
    }
}

impl LatencyClock {
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }

    /// Nanoseconds elapsed since this clock was created.
    pub fn timestamp_ns(&self) -> u64 {
        let elapsed = self.origin.elapsed().as_nanos();
        u64::try_from(elapsed).unwrap_or(u64::MAX)
    }

    /// RTT for a packet echoed with its timestamp unchanged.
    /// RTT(round-trip time): Calculate the total time elapsed for a audio packet from client to server and back
    pub fn rtt(&self, sent_ns: u64) -> Option<Duration> {
        let now_ns = self.timestamp_ns();

        now_ns
            .checked_sub(sent_ns)
            .map(Duration::from_nanos)
    }
}
