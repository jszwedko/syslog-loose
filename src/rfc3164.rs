use crate::header::Header;
///! Parsers for rfc 3164 specific formats.
use crate::parsers::{appname, hostname, u32_digits};
use crate::pri::pri;
use chrono::prelude::*;
use nom::character::complete::{space0, space1};
use nom::IResult;

/// An incomplete date is a tuple of (month, date, hour, minutes, seconds)
pub type IncompleteDate = (u32, u32, u32, u32, u32);

// The month as a three letter string. Returns the number.
fn parse_month(s: &str) -> Result<u32, String> {
    match s {
        "Jan" => Ok(1),
        "Feb" => Ok(2),
        "Mar" => Ok(3),
        "Apr" => Ok(4),
        "May" => Ok(5),
        "Jun" => Ok(6),
        "Jul" => Ok(7),
        "Aug" => Ok(8),
        "Sep" => Ok(9),
        "Oct" => Ok(10),
        "Nov" => Ok(11),
        "Dec" => Ok(12),
        _ => Err(format!("Invalid month {}", s)),
    }
}

// The timestamp for 3164 messages. MMM DD HH:MM:SS
named!(timestamp(&str) -> IncompleteDate,
       do_parse! (
           month: map_res!(take!(3), parse_month) >>
           space1 >>
           date: u32_digits >>
           space1 >>
           hour: u32_digits >>
           tag!(":") >>
           minute: u32_digits >>
           tag!(":") >>
           seconds: u32_digits >>
           ((month, date, hour, minute, seconds))
       ));

/// Makes a timestamp given all the fields of the date less the year
/// and a function to resolve the year.
fn make_timestamp<F>(
    (mon, d, h, min, s): (u32, u32, u32, u32, u32),
    get_year: F,
) -> DateTime<FixedOffset>
where
    F: FnOnce(IncompleteDate) -> i32,
{
    let year = get_year((mon, d, h, min, s));
    FixedOffset::west(0).ymd(year, mon, d).and_hms(h, min, s)
}

/// Parses the header.
/// Fails if it cant parse a 3164 format header.
pub fn header<F>(input: &str, get_year: F) -> IResult<&str, Header>
where
    F: FnOnce(IncompleteDate) -> i32,
{
    do_parse!(
        input,
        pri: pri
            >> timestamp: preceded!(space0, timestamp)
            >> hostname: opt!(preceded!(space1, hostname))
            >> appname: opt!(preceded!(space1, appname))
            >> opt!(tag!(":"))
            >> opt!(space0)
            >> (Header {
                facility: pri.0,
                severity: pri.1,
                timestamp: Some(make_timestamp(timestamp, get_year)),
                hostname: hostname.flatten(),
                version: None,
                appname: appname.flatten(),
                procid: None,
                msgid: None,
            })
    )
}

#[test]
fn parse_timestamp_3164() {
    assert_eq!(
        timestamp("Dec 28 16:49:07").unwrap(),
        ("", (12, 28, 16, 49, 7))
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pri::{SyslogFacility, SyslogSeverity};

    #[test]
    fn parse_3164_header_timestamp() {
        /*
        Note the requirement for there to be a : to separate the header and the message.
        I can't see a way around this. a is a valid hostname and message is a valid appname..
        This is not completely compliant with the RFC. 
        Are there any significant systems that will send a syslog like this?
        */
        assert_eq!(
            header("<34>Oct 11 22:14:15: a message", |_| 2019).unwrap(),
            (
                "a message",
                Header {
                    facility: Some(SyslogFacility::LOG_AUTH),
                    severity: Some(SyslogSeverity::SEV_CRIT),
                    timestamp: Some(FixedOffset::west(0).ymd(2019, 10, 11).and_hms(22, 14, 15)),
                    hostname: None,
                    version: None,
                    appname: None,
                    procid: None,
                    msgid: None,
                }
            )
        );
    }

    #[test]
    fn parse_3164_header_timestamp_host() {
        assert_eq!(
            header("<34>Oct 11 22:14:15 mymachine: a message", |_| 2019).unwrap(),
            (
                "a message",
                Header {
                    facility: Some(SyslogFacility::LOG_AUTH),
                    severity: Some(SyslogSeverity::SEV_CRIT),
                    timestamp: Some(FixedOffset::west(0).ymd(2019, 10, 11).and_hms(22, 14, 15)),
                    hostname: Some("mymachine"),
                    version: None,
                    appname: None,
                    procid: None,
                    msgid: None,
                }
            )
        );
    }
}
