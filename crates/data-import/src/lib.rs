// SokanLab data-import — Apache-2.0. Offline, tanpa jaringan.
use sokanlab_domain::{Bar, BarError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
    Empty,
    BadHeader,
    BadRow(usize),
    EmptyValue(usize, usize),
    BadBar(usize, BarError),
    NonMonotonicTime(usize),
    DuplicateTime(usize),
    /// Jumlah baris melebihi batas aman (menghindari OOM pada input raksasa).
    TooManyRows(usize),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ImportError {}

/// Batas jumlah baris data default (fail-fast, bukan OOM pada input raksasa).
/// Target desain SokanLab 10k–1M bar; batas ini sengaja jauh di atasnya.
pub const MAX_ROWS: usize = 20_000_000;

/// Format CSV MVP: timestamp,open,high,low,close,volume
/// timestamp: epoch detik ATAU milidetik integer.
pub fn parse_csv_ohlcv(text: &str) -> Result<Vec<Bar>, ImportError> {
    parse_csv_mapped(text, &ColumnMap::default(), true)
}

/// Pemetaan kolom untuk file yang urutan/namanya beda.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnMap {
    pub ts: usize,
    pub open: usize,
    pub high: usize,
    pub low: usize,
    pub close: usize,
    pub volume: usize,
    pub width: usize,
}

impl Default for ColumnMap {
    fn default() -> Self {
        Self {
            ts: 0,
            open: 1,
            high: 2,
            low: 3,
            close: 4,
            volume: 5,
            width: 6,
        }
    }
}

/// Ambil N baris data pertama (tanpa header) untuk preview wizard.
pub fn preview_csv(text: &str, n: usize) -> Vec<String> {
    text.lines()
        .skip(1)
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .take(n)
        .map(str::to_string)
        .collect()
}

pub fn parse_csv_mapped(
    text: &str,
    map: &ColumnMap,
    has_header: bool,
) -> Result<Vec<Bar>, ImportError> {
    parse_csv_mapped_with_limit(text, map, has_header, MAX_ROWS)
}

/// Sama seperti [`parse_csv_mapped`] tetapi dengan batas baris eksplisit
/// (dipakai uji & kalibrasi; default produksi memakai [`MAX_ROWS`]).
pub fn parse_csv_mapped_with_limit(
    text: &str,
    map: &ColumnMap,
    has_header: bool,
    max_rows: usize,
) -> Result<Vec<Bar>, ImportError> {
    let mut lines = text.lines();
    if has_header {
        let header = lines.next().ok_or(ImportError::Empty)?.trim();
        if header != "timestamp,open,high,low,close,volume" {
            return Err(ImportError::BadHeader);
        }
    } else if text.trim().is_empty() {
        return Err(ImportError::Empty);
    }
    let mut out = Vec::new();
    for (i, line) in lines.enumerate() {
        let row = i + 1; // 1-based setelah header
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if out.len() >= max_rows {
            return Err(ImportError::TooManyRows(row));
        }
        let p: Vec<&str> = line.split(',').collect();
        if p.len() != map.width {
            return Err(ImportError::BadRow(row));
        }
        let cell = |idx: usize| -> Result<&str, ImportError> {
            let s = p[idx].trim();
            if s.is_empty() {
                return Err(ImportError::EmptyValue(row, idx));
            }
            Ok(s)
        };
        let ts_raw: i64 = cell(map.ts)?
            .parse()
            .map_err(|_| ImportError::BadRow(row))?;
        // Dukung detik atau milidetik: < 1e12 berarti detik.
        // `unsigned_abs` aman untuk i64::MIN (abs() akan overflow/panic), dan
        // perkalian 1000 dijaga checked_mul (walau rentang ini tak bisa overflow).
        let ts_millis = if ts_raw.unsigned_abs() < 1_000_000_000_000 {
            ts_raw.checked_mul(1000).ok_or(ImportError::BadRow(row))?
        } else {
            ts_raw
        };
        let get = |idx: usize| -> Result<i64, ImportError> {
            cell(idx)?.parse().map_err(|_| ImportError::BadRow(row))
        };
        let bar = Bar {
            ts_millis,
            open_ticks: get(map.open)?,
            high_ticks: get(map.high)?,
            low_ticks: get(map.low)?,
            close_ticks: get(map.close)?,
            volume_milli: get(map.volume)?,
        };
        bar.validate().map_err(|e| ImportError::BadBar(row, e))?;
        if let Some(prev) = out.last() {
            let prev: &Bar = prev;
            if ts_millis < prev.ts_millis {
                return Err(ImportError::NonMonotonicTime(row));
            }
            if ts_millis == prev.ts_millis {
                return Err(ImportError::DuplicateTime(row));
            }
        }
        out.push(bar);
    }
    if out.is_empty() {
        return Err(ImportError::Empty);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = "timestamp,open,high,low,close,volume\n\
        1700000000,100,110,90,105,1000\n\
        1700000060,105,115,95,110,1200\n";

    #[test]
    fn parses_two_bars() {
        let bars = parse_csv_ohlcv(GOOD).unwrap();
        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0].ts_millis, 1_700_000_000_000);
    }

    #[test]
    fn bad_header_rejected() {
        assert_eq!(
            parse_csv_ohlcv("a,b,c\n").unwrap_err(),
            ImportError::BadHeader
        );
    }

    #[test]
    fn duplicate_time_rejected() {
        let dup = "timestamp,open,high,low,close,volume\n\
            1700000000,100,110,90,105,1\n\
            1700000000,100,110,90,105,1\n";
        assert!(matches!(
            parse_csv_ohlcv(dup).unwrap_err(),
            ImportError::DuplicateTime(_)
        ));
    }

    #[test]
    fn non_monotonic_rejected() {
        let rev = "timestamp,open,high,low,close,volume\n\
            1700000060,100,110,90,105,1\n\
            1700000000,100,110,90,105,1\n";
        assert!(matches!(
            parse_csv_ohlcv(rev).unwrap_err(),
            ImportError::NonMonotonicTime(_)
        ));
    }

    #[test]
    fn empty_value_rejected_with_position() {
        let bad = "timestamp,open,high,low,close,volume\n1700000000,,110,90,105,1\n";
        assert_eq!(
            parse_csv_ohlcv(bad).unwrap_err(),
            ImportError::EmptyValue(1, 1)
        );
    }

    #[test]
    fn preview_returns_first_rows() {
        let p = preview_csv(GOOD, 1);
        assert_eq!(p.len(), 1);
        assert!(p[0].starts_with("1700000000"));
    }

    #[test]
    fn row_limit_enforced() {
        let map = ColumnMap::default();
        assert!(matches!(
            parse_csv_mapped_with_limit(GOOD, &map, true, 1),
            Err(ImportError::TooManyRows(2))
        ));
        assert_eq!(
            parse_csv_mapped_with_limit(GOOD, &map, true, 2)
                .unwrap()
                .len(),
            2
        );
        // Batas lebih longgar dari isi → normal.
        assert_eq!(
            parse_csv_mapped_with_limit(GOOD, &map, true, 10)
                .unwrap()
                .len(),
            2
        );
    }
}
