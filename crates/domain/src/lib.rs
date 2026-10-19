// SokanLab domain — Apache-2.0.
// Bar memakai integer ticks agar pembukuan deterministik (tanpa f64).
// Konversi ke float hanya di layer render/analisis.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Timeframe {
    M1,
    H1,
    D1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Symbol<'a> {
    pub base: &'a str,
    pub quote: &'a str,
}

/// Satu candle OHLCV. Harga dalam ticks (mis. satoshi/sen), volume dalam milli-unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bar {
    pub ts_millis: i64,
    pub open_ticks: i64,
    pub high_ticks: i64,
    pub low_ticks: i64,
    pub close_ticks: i64,
    pub volume_milli: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarError {
    HighTooLow,
    LowTooHigh,
    NegativeVolume,
}

impl Bar {
    pub fn validate(self) -> Result<Self, BarError> {
        let hi = self.open_ticks.max(self.close_ticks);
        let lo = self.open_ticks.min(self.close_ticks);
        if self.high_ticks < hi {
            return Err(BarError::HighTooLow);
        }
        if self.low_ticks > lo {
            return Err(BarError::LowTooHigh);
        }
        if self.volume_milli < 0 {
            return Err(BarError::NegativeVolume);
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Order {
    pub side: Side,
    pub qty_milli: i64,
    pub signal_bar: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position {
    pub qty_milli: i64,
    pub cash_ticks: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_bar_passes() {
        let b = Bar {
            ts_millis: 0,
            open_ticks: 100,
            high_ticks: 110,
            low_ticks: 90,
            close_ticks: 105,
            volume_milli: 1000,
        };
        assert_eq!(b.validate().unwrap(), b);
    }

    #[test]
    fn high_below_body_fails() {
        let b = Bar {
            ts_millis: 0,
            open_ticks: 100,
            high_ticks: 99,
            low_ticks: 90,
            close_ticks: 105,
            volume_milli: 1,
        };
        assert_eq!(b.validate().unwrap_err(), BarError::HighTooLow);
    }

    #[test]
    fn negative_volume_fails() {
        let b = Bar {
            ts_millis: 0,
            open_ticks: 100,
            high_ticks: 100,
            low_ticks: 100,
            close_ticks: 100,
            volume_milli: -1,
        };
        assert_eq!(b.validate().unwrap_err(), BarError::NegativeVolume);
    }
}
