use chrono::{DateTime, Duration, NaiveDate, Timelike, Utc};
use serde::Deserialize;
use std::fs;
use std::path::Path;
use ukz_core::Ukz;

pub const EPOCH_TAI_MICROS: i64 = 1_704_067_237_000_000;
pub const UNIX_EPOCH_POSIX_MICROS: i64 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionStatus {
    Confirmed,
    Provisional,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UtcError {
    Parse(String),
    BeforeSupportedRange,
    NoLeapSecond,
    InvalidLeapSecond,
    InvalidTable(String),
    Overflow,
    Io(String),
    Json(String),
}

impl std::fmt::Display for UtcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for UtcError {}

#[derive(Debug, Clone, Deserialize)]
pub struct RawEntry {
    pub effective_utc: String,
    pub tai_minus_utc: i32,
}

#[derive(Debug, Deserialize)]
struct RawTable {
    entries: Vec<RawEntry>,
    confirmed_through: String,
}

#[derive(Debug, Clone)]
pub struct LeapEntry {
    pub effective_utc: DateTime<Utc>,
    pub tai_minus_utc: i32,
}

#[derive(Debug, Clone)]
pub struct LeapSecondTable {
    entries: Vec<LeapEntry>,
    confirmed_through: DateTime<Utc>,
}

impl LeapSecondTable {
    pub fn from_json_file(
        path: impl AsRef<Path>,
    ) -> Result<Self, UtcError> {
        let data = fs::read_to_string(path)
            .map_err(|e| UtcError::Io(e.to_string()))?;

        let raw: RawTable = serde_json::from_str(&data)
            .map_err(|e| UtcError::Json(e.to_string()))?;

        let mut entries = Vec::new();

        for entry in raw.entries {
            let dt = parse_normal(&entry.effective_utc)?;

            entries.push(LeapEntry {
                effective_utc: dt,
                tai_minus_utc: entry.tai_minus_utc,
            });
        }

        let confirmed = parse_normal(&raw.confirmed_through)?;

        Self::new(entries, confirmed)
    }

    pub fn new(
        entries: Vec<LeapEntry>,
        confirmed_through: DateTime<Utc>,
    ) -> Result<Self, UtcError> {
        let start = parse_normal("1972-01-01T00:00:00Z")?;

        if entries.is_empty() || entries[0].effective_utc != start {
            return Err(UtcError::InvalidTable(
                "Table must begin on 1972-01-01".into(),
            ));
        }

        if entries[0].tai_minus_utc != 10 {
            return Err(UtcError::InvalidTable(
                "Initial TAI-UTC offset must be 10 seconds".into(),
            ));
        }

        if confirmed_through < start {
            return Err(UtcError::InvalidTable(
                "confirmed_through is before supported range".into(),
            ));
        }

        for entry in &entries {
            if entry.effective_utc.time().num_seconds_from_midnight() != 0 {
                return Err(UtcError::InvalidTable(
                    "Leap-second transitions must occur at midnight".into(),
                ));
            }
        }

        for pair in entries.windows(2) {
            if pair[1].effective_utc <= pair[0].effective_utc {
                return Err(UtcError::InvalidTable(
                    "Entries are not strictly ordered".into(),
                ));
            }

            let difference =
                pair[1].tai_minus_utc as i64
                - pair[0].tai_minus_utc as i64;

            if difference.abs() != 1 {
                return Err(UtcError::InvalidTable(
                    "Offset change must be exactly +1 or -1 second".into(),
                ));
            }
        }

        Ok(Self {
            entries,
            confirmed_through,
        })
    }

    fn offset_at(
        &self,
        dt: DateTime<Utc>,
    ) -> Result<(i32, ConversionStatus), UtcError> {
        if dt < self.entries[0].effective_utc {
            return Err(UtcError::BeforeSupportedRange);
        }

        let mut offset = self.entries[0].tai_minus_utc;

        for entry in self.entries.iter().skip(1) {
            if dt < entry.effective_utc {
                break;
            }

            offset = entry.tai_minus_utc;
        }

        let status = if dt <= self.confirmed_through {
            ConversionStatus::Confirmed
        } else {
            ConversionStatus::Provisional
        };

        Ok((offset, status))
    }

    fn transition_after_day(
        &self,
        day: NaiveDate,
    ) -> Option<(i32, i32)> {
        let next_day = day
            .succ_opt()?
            .and_hms_opt(0, 0, 0)?;

        let next = DateTime::<Utc>::from_naive_utc_and_offset(
            next_day,
            Utc,
        );

        for index in 1..self.entries.len() {
            if self.entries[index].effective_utc == next {
                return Some((
                    self.entries[index - 1].tai_minus_utc,
                    self.entries[index].tai_minus_utc,
                ));
            }
        }

        None
    }
}

pub struct TaiAdapter;

impl TaiAdapter {
    pub fn from_tai_micros(
        value: i64,
    ) -> Result<Ukz, UtcError> {
        let relative = value
            .checked_sub(EPOCH_TAI_MICROS)
            .ok_or(UtcError::Overflow)?;

        Ok(Ukz::new(
            relative.div_euclid(Ukz::TICK_MICROS),
        ))
    }
}

pub struct UtcAdapter;

impl UtcAdapter {
    pub fn from_utc_string(
        input: &str,
        table: &LeapSecondTable,
    ) -> Result<(Ukz, ConversionStatus), UtcError> {
        let (base, micros, leap) = parse_utc_parts(input)?;

        if leap {
            if base.hour() != 23 || base.minute() != 59 {
                return Err(UtcError::InvalidLeapSecond);
            }

            let day = base.date_naive();

            let (old_offset, new_offset) = table
                .transition_after_day(day)
                .ok_or(UtcError::NoLeapSecond)?;

            if new_offset != old_offset + 1 {
                return Err(UtcError::InvalidLeapSecond);
            }

            let next_day = day
                .succ_opt()
                .ok_or(UtcError::Overflow)?
                .and_hms_opt(0, 0, 0)
                .ok_or(UtcError::Overflow)?;

            let next = DateTime::<Utc>::from_naive_utc_and_offset(
                next_day,
                Utc,
            );

            let unix_micros = next
                .timestamp_micros()
                .checked_add(micros as i64)
                .ok_or(UtcError::Overflow)?;

            let tai_micros = unix_micros
                .checked_add((old_offset as i64) * 1_000_000)
                .ok_or(UtcError::Overflow)?;

            let status = if next <= table.confirmed_through {
                ConversionStatus::Confirmed
            } else {
                ConversionStatus::Provisional
            };

            return Ok((
                TaiAdapter::from_tai_micros(tai_micros)?,
                status,
            ));
        }

        let dt = base
            .checked_add_signed(Duration::microseconds(micros as i64))
            .ok_or(UtcError::Overflow)?;

        let (offset, status) = table.offset_at(dt)?;

        let tai_micros = dt
            .timestamp_micros()
            .checked_add((offset as i64) * 1_000_000)
            .ok_or(UtcError::Overflow)?;

        Ok((
            TaiAdapter::from_tai_micros(tai_micros)?,
            status,
        ))
    }
}

fn parse_normal(
    input: &str,
) -> Result<DateTime<Utc>, UtcError> {
    DateTime::parse_from_rfc3339(input)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| UtcError::Parse(e.to_string()))
}

fn parse_utc_parts(
    input: &str,
) -> Result<(DateTime<Utc>, u32, bool), UtcError> {
    if !input.ends_with('Z') {
        return Err(UtcError::Parse(
            "UTC input must end in Z".into(),
        ));
    }

    let body = &input[..input.len() - 1];

    let (whole, fraction) = match body.split_once('.') {
        Some((whole, fraction)) => {
            if fraction.is_empty() {
                return Err(UtcError::Parse(
                    "Fractional part must not be empty".into(),
                ));
            }

            (whole, fraction)
        }
        None => (body, ""),
    };

    if fraction.len() > 6
        || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(UtcError::Parse(
            "Fraction must contain 1 to 6 digits".into(),
        ));
    }

    let micros = if fraction.is_empty() {
        0
    } else {
        let value = fraction
            .parse::<u32>()
            .map_err(|e| UtcError::Parse(e.to_string()))?;

        value * 10u32.pow((6 - fraction.len()) as u32)
    };

    let leap = whole.ends_with(":60");

    let normalized = if leap {
        format!("{}59Z", &whole[..whole.len() - 2])
    } else {
        format!("{}Z", whole)
    };

    let dt = parse_normal(&normalized)?;

    Ok((dt, micros, leap))
}
