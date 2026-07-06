pub use jiff::Timestamp;
pub use std::time::{Duration, Instant};

use std::ops;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Time {
    pub timestamp: Timestamp,
    pub instant: Instant,
}

impl Time {
    pub fn now() -> Self {
        Self {
            timestamp: jiff::Timestamp::now(),
            instant: Instant::now(),
        }
    }
}

impl ops::Sub<Time> for Time {
    type Output = Duration;

    fn sub(self, rhs: Time) -> Self::Output {
        self.instant - rhs.instant
    }
}

impl ops::Sub<Time> for Instant {
    type Output = Duration;

    fn sub(self, rhs: Time) -> Self::Output {
        self - rhs.instant
    }
}
