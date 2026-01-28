/// A simple duration type representing time durations in ticks (nanoseconds)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration(u64);

impl Duration {
    /// Create a new Duration from ticks (nanoseconds)
    pub const fn from_u64(ticks: u64) -> Self {
        Self(ticks)
    }

    /// Get the duration in ticks (nanoseconds)
    pub const fn as_u64(&self) -> u64 {
        self.0
    }

    /// Create a new Duration from nanoseconds
    pub const fn from_nanoseconds(nanoseconds: u64) -> Self {
        return Self::from_u64(nanoseconds * Self::NanoSecond.as_u64());
    }

    /// Create a new Duration from microseconds
    pub const fn from_microseconds(microseconds: u64) -> Self {
        return Self::from_u64(microseconds * Self::MicroSecond.as_u64());
    }

    /// Create a new Duration from milliseconds
    pub const fn from_milliseconds(milliseconds: u64) -> Self {
        return Self::from_u64(milliseconds * Self::MilliSecond.as_u64());
    }

    /// Create a new Duration from seconds
    pub const fn from_seconds(seconds: u64) -> Self {
        return Self::from_u64(seconds * Self::Second.as_u64());
    }
}

#[allow(non_upper_case_globals)]
impl Duration {
    /// Zero duration
    pub const Zero: Self = Self::from_u64(0);

    pub const NanoSecond: Self = Self::from_u64(1);
    pub const MicroSecond: Self = Self::from_u64(1_000 * Self::NanoSecond.as_u64());
    pub const MilliSecond: Self = Self::from_u64(1_000 * Self::MicroSecond.as_u64());
    pub const Second: Self = Self::from_u64(1_000 * Self::MilliSecond.as_u64());
    pub const Minute: Self = Self::from_u64(60 * Self::Second.as_u64());
    pub const Hour: Self = Self::from_u64(60 * Self::Minute.as_u64());
    pub const Day: Self = Self::from_u64(24 * Self::Hour.as_u64());
}
