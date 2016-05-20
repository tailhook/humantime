use std::str::Chars;
use std::time::Duration;
use std::error::Error as StdError;

quick_error! {
    /// Error parsing human-friendly duration
    #[derive(Debug, PartialEq)]
    pub enum Error {
        /// Invalid character during parsing
        ///
        /// More specifically anything that is not alphanumeric is prohibited
        ///
        /// The field is an byte offset of the character in the string.
        InvalidCharacter(offset: usize) {
            display("invalid character at {}", offset)
            description("invalid character")
        }
        /// Non-numeric value where number is expected
        ///
        /// This usually means that either time unit is broken into words,
        /// e.g. `m sec` instead of `msec`, or just number is omitted,
        /// for example `2 hours min` instead of `2 hours 1 min`
        ///
        /// The field is an byte offset of the errorneous character
        /// in the string.
        NumberExpected(offset: usize) {
            display("expected number at {}", offset)
            description("expected number")
        }
        /// Unit in the number is not one of allowed units
        ///
        /// See documentation of `parse_duration` for the list of supported
        /// time units.
        ///
        /// The two fields are start and end (exclusive) of the slice from
        /// the original string, containing errorneous value
        UnknownUnit(start: usize, end: usize) {
            display("unknown unit at {}-{}", start, end)
            description("unknown unit")
        }
        /// The numeric value is too large
        ///
        /// Usually this means value is too large to be useful. If user writes
        /// data in subsecond units. Then the maximum is about 3k years. When
        /// using seconds, or larger, the limit is even larger.
        NumberOverflow {
            display(self_) -> ("{}", self_.description())
            description("number is too large")
        }
        /// The value was an empty string (or only whitespace)
        Empty {
            display(self_) -> ("{}", self_.description())
            description("value was empty")
        }
    }
}

trait OverflowOp: Sized {
    fn mul(self, other: Self) -> Result<Self, Error>;
    fn add(self, other: Self) -> Result<Self, Error>;
}

impl OverflowOp for u64 {
    fn mul(self, other: Self) -> Result<Self, Error> {
        self.checked_mul(other).ok_or(Error::NumberOverflow)
    }
    fn add(self, other: Self) -> Result<Self, Error> {
        self.checked_add(other).ok_or(Error::NumberOverflow)
    }
}

struct Parser<'a> {
    iter: Chars<'a>,
    src: &'a str,
    current: (u64, u64),
}

impl<'a> Parser<'a> {
    fn off(&self) -> usize {
        self.src.len() - self.iter.as_str().len()
    }

    fn parse_first_char(&mut self) -> Result<Option<u64>, Error> {
        let off = self.off();
        for c in self.iter.by_ref() {
            match c {
                '0'...'9' => {
                    return Ok(Some(c as u64 - '0' as u64));
                }
                c if c.is_whitespace() => continue,
                _ => {
                    return Err(Error::NumberExpected(off));
                }
            }
        }
        return Ok(None);
    }
    fn parse_unit(&mut self, n: u64, start: usize, end: usize)
        -> Result<(), Error>
    {
        let (mut sec, nsec) = match &self.src[start..end] {
            "nsec" | "ns" => (0u64, n),
            "usec" | "us" => (0u64, try!(n.mul(1000))),
            "msec" | "ms" => (0u64, try!(n.mul(1000_000))),
            "seconds" | "second" | "sec" | "s" => (n, 0),
            "minutes" | "minute" | "min" | "m" => (try!(n.mul(60)), 0),
            "hours" | "hour" | "hr" | "h" => (try!(n.mul(3600)), 0),
            "days" | "day" | "d" => (try!(n.mul(86400)), 0),
            "weeks" | "week" | "w" => (try!(n.mul(86400*7)), 0),
            "months" | "month" | "M" => (try!(n.mul(2630016)), 0), // 30.44d
            "years" | "year" | "y" => (try!(n.mul(31557600)), 0), // 365.25d
            _ => return Err(Error::UnknownUnit(start, end)),
        };
        let mut nsec = try!(self.current.1.add(nsec));
        if nsec > 1000_000_000 {
            sec = try!(sec.add(nsec / 1000_000_000));
            nsec %= 1000_000_000;
        }
        sec = try!(self.current.0.add(sec));
        self.current = (sec, nsec);
        Ok(())
    }

    fn parse(mut self) -> Result<Duration, Error> {
        let mut n = try!(try!(self.parse_first_char()).ok_or(Error::Empty));
        'outer: loop {
            let mut off = self.off();
            while let Some(c) = self.iter.next() {
                match c {
                    '0'...'9' => {
                        n = try!(n.checked_mul(10)
                            .and_then(|x| x.checked_add(c as u64 - '0' as u64))
                            .ok_or(Error::NumberOverflow));
                    }
                    c if c.is_whitespace() => {}
                    'a'...'z' | 'A'...'Z' => {
                        break;
                    }
                    _ => {
                        return Err(Error::InvalidCharacter(off));
                    }
                }
                off = self.off();
            }
            let start = off;
            let mut off = self.off();
            while let Some(c) = self.iter.next() {
                match c {
                    '0'...'9' => {
                        try!(self.parse_unit(n, start, off));
                        n = c as u64 - '0' as u64;
                        continue 'outer;
                    }
                    c if c.is_whitespace() => break,
                    'a'...'z' | 'A'...'Z' => {}
                    _ => {
                        return Err(Error::InvalidCharacter(off));
                    }
                }
                off = self.off();
            }
            try!(self.parse_unit(n, start, off));
            n = match try!(self.parse_first_char()) {
                Some(n) => n,
                None => return Ok(
                    Duration::new(self.current.0, self.current.1 as u32)),
            };
        }
    }

}

/// Parse duration object
///
/// The duration object is a concatenation of time spans. Where each time
/// span is an integer number and a suffix. Supported suffixes:
/// * `nsec`, `ns` -- microseconds
/// * `usec`, `us` -- microseconds
/// * `msec`, `ms` -- milliseconds
/// * `seconds`, `second`, `sec`, `s`
/// * `minutes`, `minute`, `min`, `m`
/// * `hours`, `hour`, `hr`, `h`
/// * `days`, `day`, `d`
/// * `weeks`, `week`, `w`
/// * `months`, `month`, `M` -- defined as 30.44 days
/// * `years`, `year`, `y` -- defined as 365.25 days
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use humantime::parse_duration;
///
/// assert_eq!(parse_duration("2h 37min"), Ok(Duration::new(9420, 0)));
/// assert_eq!(parse_duration("32ms"), Ok(Duration::new(0, 32_000_000)));
/// ```
pub fn parse_duration(s: &str) -> Result<Duration, Error> {
    Parser {
        iter: s.chars(),
        src: s,
        current: (0, 0),
    }.parse()
}

#[test]
fn test_units() {
    assert_eq!(parse_duration("17nsec"), Ok(Duration::new(0, 17)));
    assert_eq!(parse_duration("33ns"), Ok(Duration::new(0, 33)));
    assert_eq!(parse_duration("3usec"), Ok(Duration::new(0, 3000)));
    assert_eq!(parse_duration("78us"), Ok(Duration::new(0, 78000)));
    assert_eq!(parse_duration("31msec"), Ok(Duration::new(0, 31000000)));
    assert_eq!(parse_duration("6ms"), Ok(Duration::new(0, 6000000)));
    assert_eq!(parse_duration("3000s"), Ok(Duration::new(3000, 0)));
    assert_eq!(parse_duration("300sec"), Ok(Duration::new(300, 0)));
    assert_eq!(parse_duration("50seconds"), Ok(Duration::new(50, 0)));
    assert_eq!(parse_duration("1second"), Ok(Duration::new(1, 0)));
    assert_eq!(parse_duration("100m"), Ok(Duration::new(6000, 0)));
    assert_eq!(parse_duration("12min"), Ok(Duration::new(720, 0)));
    assert_eq!(parse_duration("1minute"), Ok(Duration::new(60, 0)));
    assert_eq!(parse_duration("7minutes"), Ok(Duration::new(420, 0)));
    assert_eq!(parse_duration("2h"), Ok(Duration::new(7200, 0)));
    assert_eq!(parse_duration("7hr"), Ok(Duration::new(25200, 0)));
    assert_eq!(parse_duration("1hour"), Ok(Duration::new(3600, 0)));
    assert_eq!(parse_duration("24hours"), Ok(Duration::new(86400, 0)));
    assert_eq!(parse_duration("1day"), Ok(Duration::new(86400, 0)));
    assert_eq!(parse_duration("2days"), Ok(Duration::new(172800, 0)));
    assert_eq!(parse_duration("365d"), Ok(Duration::new(31536000, 0)));
    assert_eq!(parse_duration("1week"), Ok(Duration::new(604800, 0)));
    assert_eq!(parse_duration("7weeks"), Ok(Duration::new(4233600, 0)));
    assert_eq!(parse_duration("52w"), Ok(Duration::new(31449600, 0)));
    assert_eq!(parse_duration("1month"), Ok(Duration::new(2630016, 0)));
    assert_eq!(parse_duration("3months"), Ok(Duration::new(3*2630016, 0)));
    assert_eq!(parse_duration("12M"), Ok(Duration::new(31560192, 0)));
    assert_eq!(parse_duration("1year"), Ok(Duration::new(31557600, 0)));
    assert_eq!(parse_duration("7years"), Ok(Duration::new(7*31557600, 0)));
    assert_eq!(parse_duration("17y"), Ok(Duration::new(536479200, 0)));
}

#[test]
fn test_combo() {
    assert_eq!(parse_duration("20 min 17 nsec "), Ok(Duration::new(1200, 17)));
    assert_eq!(parse_duration("2h 15m"), Ok(Duration::new(8100, 0)));
}

#[test]
fn test_overlow() {
    // Overflow on subseconds is earlier because of how we do conversion
    // we could fix it, but I don't see any good reason for this
    assert_eq!(parse_duration("100000000000000000000ns"),
        Err(Error::NumberOverflow));
    assert_eq!(parse_duration("100000000000000000us"),
        Err(Error::NumberOverflow));
    assert_eq!(parse_duration("100000000000000ms"),
        Err(Error::NumberOverflow));

    assert_eq!(parse_duration("100000000000000000000s"),
        Err(Error::NumberOverflow));
    assert_eq!(parse_duration("10000000000000000000m"),
        Err(Error::NumberOverflow));
    assert_eq!(parse_duration("1000000000000000000h"),
        Err(Error::NumberOverflow));
    assert_eq!(parse_duration("100000000000000000d"),
        Err(Error::NumberOverflow));
    assert_eq!(parse_duration("10000000000000000w"),
        Err(Error::NumberOverflow));
    assert_eq!(parse_duration("1000000000000000M"),
        Err(Error::NumberOverflow));
    assert_eq!(parse_duration("10000000000000y"),
        Err(Error::NumberOverflow));
}
