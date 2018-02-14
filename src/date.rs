use std::time::{SystemTime, Duration, UNIX_EPOCH};


quick_error! {
    /// Error parsing human-friendly datetime
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub enum Error {
        OutOfRange {
            display("numeric component is out of range")
        }
        InvalidDigit {
            display("bad character where digit is expected")
        }
        InvalidFormat {
            display("datetime format is invalid")
        }
    }
}

#[inline]
fn two_digits(b1: u8, b2: u8) -> Result<u64, Error> {
    if b1 < b'0' || b2 < b'0' || b1 > b'9' || b2 > b'9' {
        return Err(Error::InvalidDigit);
    }
    Ok(((b1 - b'0')*10 + (b2 - b'0')) as u64)
}

/// Parse ISO datetime ``2018-02-14T00:28:07Z``
pub fn parse_iso_datetime_seconds(s: &str) -> Result<SystemTime, Error> {
    if s.len() != "2018-02-14T00:28:07Z".len() {
        return Err(Error::InvalidFormat);
    }
    let b = s.as_bytes();  // for careless slicing
    if b[4] != b'-' || b[7] != b'-' || b[10] != b'T' ||
       b[13] != b':' || b[16] != b':' || b[19] != b'Z'
    {
        return Err(Error::InvalidFormat);
    }
    let year = two_digits(b[0], b[1])? * 100 + two_digits(b[2], b[3])?;
    let month = two_digits(b[5], b[6])?;
    let day = two_digits(b[8], b[9])?;
    let hour = two_digits(b[11], b[12])?;
    let minute = two_digits(b[14], b[15])?;
    let second = two_digits(b[17], b[18])?;

    if year < 1970 {
        return Err(Error::OutOfRange);
    }
    let leap_years = ((year - 1) - 1968) / 4 - ((year - 1) - 1900) / 100 +
                     ((year - 1) - 1600) / 400;
    let leap = is_leap_year(year);
    let (mut ydays, mdays) = match month {
        1 => (0, 31),
        2 if leap => (31, 29),
        2 => (31, 28),
        3 => (59, 31),
        4 => (90, 30),
        5 => (120, 31),
        6 => (151, 30),
        7 => (181, 31),
        8 => (212, 31),
        9 => (243, 30),
        10 => (273, 31),
        11 => (304, 30),
        12 => (334, 31),
        _ => return Err(Error::OutOfRange),
    };
    ydays += day - 1;
    if is_leap_year(year) && month > 2 {
        ydays += 1;
    }
    let days = (year - 1970) * 365 + leap_years + ydays;

    if day > mdays || day == 0 {
        return Err(Error::OutOfRange);
    }

    let time = second + minute * 60 + hour * 3600;
    return Ok(UNIX_EPOCH + Duration::from_secs(time + days * 86400));
}

fn is_leap_year(y: u64) -> bool {
    y % 4 == 0 && (!(y % 100 == 0) || y % 400 == 0)
}

#[cfg(test)]
mod test {
    extern crate time;
    extern crate rand;
    use self::rand::Rng;
    use std::time::{UNIX_EPOCH, SystemTime, Duration};
    use super::parse_iso_datetime_seconds;

    fn from_sec(sec: u64) -> (String, SystemTime) {
        let s = time::at_utc(time::Timespec { sec: sec as i64, nsec: 0 })
                  .rfc3339().to_string();
        let time = UNIX_EPOCH + Duration::new(sec, 0);
        return (s, time)
    }

    #[test]
    fn smoke_tests() {
        assert_eq!(parse_iso_datetime_seconds("2018-02-13T23:08:32Z").unwrap(),
                   UNIX_EPOCH + Duration::new(1518563312, 0));
        assert_eq!(parse_iso_datetime_seconds("2012-01-01T00:00:00Z").unwrap(),
                   UNIX_EPOCH + Duration::new(1325376000, 0));
    }
    #[test]
    fn upper_bound() {
        assert_eq!(parse_iso_datetime_seconds("9999-12-31T23:59:59Z").unwrap(),
                   UNIX_EPOCH + Duration::new(253402300800-1, 0));
    }

    #[test]
    fn first_731_days() {
        let year_start = 0;  // 1970
        for day in 0.. (365 * 2 + 1) {  // scan leap year and non-leap year
            let (s, time) = from_sec(year_start + day * 86400);
            assert_eq!(parse_iso_datetime_seconds(&s).unwrap(), time);
        }
    }

    #[test]
    fn the_731_consecutive_days() {
        let year_start = 1325376000;  // 2012
        for day in 0.. (365 * 2 + 1) {  // scan leap year and non-leap year
            let (s, time) = from_sec(year_start + day * 86400);
            assert_eq!(parse_iso_datetime_seconds(&s).unwrap(), time);
        }
    }

    #[test]
    fn all_86400_seconds() {
        let day_start = 1325376000;
        for second in 0..86400 {  // scan leap year and non-leap year
            let (s, time) = from_sec(day_start + second);
            assert_eq!(parse_iso_datetime_seconds(&s).unwrap(), time);
        }
    }

    #[test]
    fn random_past() {
        let upper = SystemTime::now().duration_since(UNIX_EPOCH).unwrap()
            .as_secs();
        for _ in 0..10000 {
            let sec = rand::thread_rng().gen_range(0, upper);
            let (s, time) = from_sec(sec);
            assert_eq!(parse_iso_datetime_seconds(&s).unwrap(), time);
        }
    }

    #[test]
    fn random_wide_range() {
        for _ in 0..10000 {
            let sec = rand::thread_rng().gen_range(0, 253370764800);
            let (s, time) = from_sec(sec);
            assert_eq!(parse_iso_datetime_seconds(&s).unwrap(), time);
        }
    }

}
