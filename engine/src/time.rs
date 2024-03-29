use std::time::Duration;

use crate::search::SearchResult;

pub trait TimeManager {
    ///Update the time manager's internal state with a new result.
    ///`time` represents the duration since the last update.
    ///Returns a timeout to the next update; If no update happens before
    ///the timeout, stop searching.
    fn update(&mut self, result: &SearchResult, time: Duration) -> Duration;
}

///Extremely naive time manager that only uses a fixed amount of time per move.
pub struct FixedTimeManager {
    interval: Duration,
    elapsed: Duration
}

impl FixedTimeManager {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            elapsed: Duration::ZERO
        }
    }
}

impl TimeManager for FixedTimeManager {
    fn update(&mut self, _: &SearchResult, time: Duration) -> Duration {
        self.elapsed += time;
        if self.interval > self.elapsed {
            self.interval - self.elapsed
        } else {
            Duration::ZERO
        }
    }
}

///Extremely naive time manager that only uses a fixed percentage of time per move
pub struct PercentageTimeManager(FixedTimeManager);

impl PercentageTimeManager {
    pub fn new(time_left: Duration, percentage: f32, minimum_time: Duration) -> Self {
        Self(FixedTimeManager::new(time_left.mul_f32(percentage).max(minimum_time)))
    }
}

impl TimeManager for PercentageTimeManager {
    fn update(&mut self, result: &SearchResult, time: Duration) -> Duration {
        self.0.update(result, time)
    }
}

///The standard time manager. Still quite naive.
pub enum StandardTimeManager {
    Infinite,
    Fixed(Duration),
    Standard {
        allocated: Duration,
        max_usage: Duration,
        elapsed: Duration
    }
}

impl StandardTimeManager {
    pub fn standard(time_left: Duration, increment: Duration) -> Self {
        Self::Standard {
            allocated: (time_left + increment).mul_f32(0.025).min(time_left),
            max_usage: time_left / 3,
            elapsed: Duration::ZERO
        }
    }
}

impl TimeManager for StandardTimeManager {
    fn update(&mut self, _: &SearchResult, time_since_update: Duration) -> Duration {
        match self {
            Self::Infinite => Duration::MAX,
            Self::Fixed(time_left) => {
                *time_left = time_left.saturating_sub(time_since_update);
                *time_left
            }
            Self::Standard {
                allocated,
                max_usage,
                elapsed
            } => {
                *elapsed += time_since_update;
                if elapsed >= allocated {
                    *max_usage = Duration::ZERO;
                }
                max_usage.saturating_sub(*elapsed)
            }
        }
    }
}
