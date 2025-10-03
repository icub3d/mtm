use std::error::Error;
use std::fmt;
use std::thread;
use std::time::Duration;

use clap::Parser;
use enigo::{Enigo, MouseControllable};
use rand::Rng;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Wiggle the mouse at random intervals to keep the screen awake.",
    long_about = None
)]
struct Args {
    /// Shortest allowed idle interval between wiggles (e.g. 30s, 4m2s).
    #[arg(
        short = 'l',
        long = "lower",
        default_value = "45s",
        value_parser = parse_duration_arg
    )]
    lower: Duration,

    /// Longest allowed idle interval between wiggles (e.g. 60s, 5m15s).
    #[arg(
        short = 'u',
        long = "upper",
        default_value = "90s",
        value_parser = parse_duration_arg
    )]
    upper: Duration,

    /// Maximum pixels to nudge the pointer in any direction.
    #[arg(
        short = 'd',
        long = "distance",
        default_value_t = 15u32,
        value_parser = clap::value_parser!(u32).range(1..=250)
    )]
    distance: u32,

    /// Enable verbose logging about waits and mouse movement.
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    if args.lower > args.upper {
        return Err("Lower interval must be less than or equal to upper interval.".into());
    }

    if args.verbose {
        println!(
            "Running with interval between {:?} and {:?}; moving up to {}px",
            args.lower, args.upper, args.distance
        );
    }

    run_loop(args)?;
    Ok(())
}

fn run_loop(args: Args) -> Result<(), Box<dyn Error>> {
    let mut rng = rand::thread_rng();
    let mut enigo = Enigo::new();

    loop {
        let wait = random_duration_between(&mut rng, args.lower, args.upper)?;

        if args.verbose {
            println!("Sleeping for {:?} before next wiggle", wait);
        }

        thread::sleep(wait);

        let delta_x = random_offset(&mut rng, args.distance);
        let delta_y = random_offset(&mut rng, args.distance);

        if delta_x == 0 && delta_y == 0 {
            if args.verbose {
                println!("Skipped wiggle because the random offset was (0, 0); retrying");
            }
            continue;
        }

        if args.verbose {
            println!("Moving mouse by ({delta_x}, {delta_y}) and then returning it to position");
        }

        enigo.mouse_move_relative(delta_x, delta_y);
        thread::sleep(Duration::from_millis(100));
        enigo.mouse_move_relative(-delta_x, -delta_y);

        if args.verbose {
            println!("Mouse returned to its original position");
        }
    }
}

fn random_offset(rng: &mut impl Rng, max_distance: u32) -> i32 {
    let range = -(max_distance as i32)..=(max_distance as i32);
    rng.gen_range(range)
}

fn random_duration_between(
    rng: &mut impl Rng,
    lower: Duration,
    upper: Duration,
) -> Result<Duration, Box<dyn Error>> {
    if lower == upper {
        return Ok(lower);
    }

    let delta = upper
        .checked_sub(lower)
        .ok_or("Upper interval must be greater than or equal to lower interval.")?;

    let delta_ms = delta.as_millis();
    if delta_ms == 0 {
        return Ok(lower);
    }

    if delta_ms > u64::MAX as u128 {
        return Err("Interval range is too large.".into());
    }

    let jitter_ms = rng.gen_range(0..=delta_ms as u64);
    Ok(lower + Duration::from_millis(jitter_ms))
}

fn parse_duration_arg(value: &str) -> Result<Duration, String> {
    parse_duration(value).map_err(|err| err.to_string())
}

fn parse_duration(input: &str) -> Result<Duration, ParseDurationError> {
    let mut chars = input.chars().peekable();
    let mut total = Duration::default();
    let mut saw_value = false;

    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }

        let number = read_number(&mut chars)?;
        skip_whitespace(&mut chars);
        let unit = read_unit(&mut chars)?;
        let part = unit.to_duration(number)?;

        total = total
            .checked_add(part)
            .ok_or(ParseDurationError::DurationOverflow)?;
        saw_value = true;
    }

    if !saw_value {
        return Err(ParseDurationError::Empty);
    }

    Ok(total)
}

fn read_number(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Result<u64, ParseDurationError> {
    let mut value: u64 = 0;
    let mut saw_digit = false;

    while let Some(&ch) = chars.peek() {
        if ch.is_ascii_digit() {
            saw_digit = true;
            value = value
                .checked_mul(10)
                .and_then(|v| v.checked_add((ch as u8 - b'0') as u64))
                .ok_or(ParseDurationError::NumberOverflow)?;
            chars.next();
        } else {
            break;
        }
    }

    if !saw_digit {
        return Err(ParseDurationError::ExpectedNumber);
    }

    Ok(value)
}

fn skip_whitespace(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    while matches!(chars.peek(), Some(ch) if ch.is_whitespace()) {
        chars.next();
    }
}

fn read_unit(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Result<DurationUnit, ParseDurationError> {
    let mut letters = String::new();

    while let Some(&ch) = chars.peek() {
        if ch.is_ascii_alphabetic() {
            letters.push(ch.to_ascii_lowercase());
            chars.next();
        } else {
            break;
        }
    }

    if letters.is_empty() {
        return Err(ParseDurationError::MissingUnit);
    }

    DurationUnit::from_str(&letters)
}

#[derive(Debug, Clone, Copy)]
enum DurationUnit {
    Hours,
    Minutes,
    Seconds,
    Milliseconds,
}

impl DurationUnit {
    fn from_str(value: &str) -> Result<Self, ParseDurationError> {
        match value {
            "h" => Ok(Self::Hours),
            "m" => Ok(Self::Minutes),
            "s" => Ok(Self::Seconds),
            "ms" => Ok(Self::Milliseconds),
            _ => Err(ParseDurationError::InvalidUnit(value.to_string())),
        }
    }

    fn to_duration(self, amount: u64) -> Result<Duration, ParseDurationError> {
        match self {
            Self::Hours => amount
                .checked_mul(3_600)
                .map(Duration::from_secs)
                .ok_or(ParseDurationError::DurationOverflow),
            Self::Minutes => amount
                .checked_mul(60)
                .map(Duration::from_secs)
                .ok_or(ParseDurationError::DurationOverflow),
            Self::Seconds => Ok(Duration::from_secs(amount)),
            Self::Milliseconds => Ok(Duration::from_millis(amount)),
        }
    }
}

#[derive(Debug)]
enum ParseDurationError {
    Empty,
    ExpectedNumber,
    MissingUnit,
    InvalidUnit(String),
    NumberOverflow,
    DurationOverflow,
}

impl fmt::Display for ParseDurationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "Expected a duration value (e.g. 30s, 4m2s)."),
            Self::ExpectedNumber => write!(f, "Expected digits before the unit."),
            Self::MissingUnit => write!(f, "Missing unit (expected h, m, s, or ms)."),
            Self::InvalidUnit(unit) => write!(f, "Unrecognized unit '{unit}'. Use h, m, s, or ms."),
            Self::NumberOverflow => write!(f, "Number is too large."),
            Self::DurationOverflow => write!(f, "Duration is too large."),
        }
    }
}

impl Error for ParseDurationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_compound_duration() {
        let duration = parse_duration("1h2m3s").unwrap();
        assert_eq!(duration, Duration::from_secs(3_723));
    }

    #[test]
    fn rejects_missing_units() {
        assert!(matches!(
            parse_duration("123"),
            Err(ParseDurationError::MissingUnit)
        ));
    }

    #[test]
    fn parses_with_milliseconds() {
        let duration = parse_duration("500ms").unwrap();
        assert_eq!(duration, Duration::from_millis(500));
    }
}
