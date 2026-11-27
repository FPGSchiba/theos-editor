//! Reader for the answer of `git log --name-status -z` with the format of [`super::FORMAT`].
//!
//! Each commit is, as git 2.56 writes it: `0x1e`, six fields separated by `0x1f`, a NUL (the
//! end of the format under `-z`), a newline, then its name-status line as NUL-terminated
//! fields: the letter with its score (`R083`), the path, and for a rename or copy the new path.
//! The answer is read in that order, so a path may hold any byte but NUL; a header with the
//! wrong number of fields is an error, never a guess.

use super::{Commit, Person, Trailer};
use crate::status::Change;
use crate::status::parse::change;

/// Starts a commit.
const RECORD: u8 = 0x1e;
/// Separates the fields of a commit's header.
const UNIT: u8 = 0x1f;
/// Separates two trailers.
const TRAILERS: u8 = 0x1d;
/// Separates a trailer's key from its value.
const KEY_VALUE: u8 = 0x1c;
/// The fields of a header: id, name, email, date, subject, trailers.
const HEADER_FIELDS: usize = 6;

/// Reads every commit of the answer.
pub(crate) fn commits(out: &[u8]) -> Result<Vec<Commit>, String> {
    let mut rest = out;
    let mut commits = Vec::new();
    while !rest.is_empty() {
        let (commit, after) = commit(rest)?;
        commits.push(commit);
        rest = after;
    }
    Ok(commits)
}

/// Reads the commit at the start of `input` and returns it with the input after it.
fn commit(input: &[u8]) -> Result<(Commit, &[u8]), String> {
    let input = input
        .strip_prefix(&[RECORD])
        .ok_or("a commit does not start with 0x1e")?;
    let (header, rest) = field(input)?;
    let rest = rest
        .strip_prefix(
            b"
",
        )
        .unwrap_or(rest);
    let (letters, rest) = field(rest)?;
    let letter = *letters.first().ok_or("a commit has an empty change")?;
    let change = change(letter)?;
    let (first, rest) = field(rest)?;
    let (path, from, rest) = if matches!(change, Change::Renamed | Change::Copied) {
        let (second, rest) = field(rest)?;
        (second, Some(first.to_vec()), rest)
    } else {
        (first, None, rest)
    };
    let fields: Vec<&[u8]> = header.split(|byte| *byte == UNIT).collect();
    let [id, name, email, date, subject, trailers] = fields[..] else {
        return Err(format!(
            "a commit has {} header fields, not {HEADER_FIELDS}",
            fields.len()
        ));
    };
    let (time, offset_minutes) = raw_date(date)?;
    let commit = Commit {
        id: String::from_utf8(id.to_vec()).map_err(|_| "a commit id is not UTF-8")?,
        author: Person {
            name: text(name),
            email: text(email),
        },
        time,
        offset_minutes,
        subject: text(subject),
        trailers: trailer_list(trailers)?,
        change,
        path: path.to_vec(),
        from,
    };
    Ok((commit, rest))
}

/// The bytes up to the next NUL, and the input after it.
fn field(input: &[u8]) -> Result<(&[u8], &[u8]), String> {
    let end = input
        .iter()
        .position(|byte| *byte == 0)
        .ok_or("a field does not end with a NUL")?;
    Ok((&input[..end], &input[end + 1..]))
}

/// Text that git re-encodes to UTF-8; a commit without an encoding header may hold other bytes,
/// which are read lossily rather than failing the whole history.
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// `--date=raw`: `<seconds> <±hhmm>`, as seconds and minutes east of UTC.
fn raw_date(date: &[u8]) -> Result<(i64, i32), String> {
    let wrong = || format!("`{}` is not a raw date", String::from_utf8_lossy(date));
    let date = std::str::from_utf8(date).map_err(|_| wrong())?;
    let (seconds, zone) = date.split_once(' ').ok_or_else(wrong)?;
    let seconds: i64 = seconds.parse().map_err(|_| wrong())?;
    let (sign, digits) = match zone.split_at_checked(1) {
        Some(("+", digits)) => (1, digits),
        Some(("-", digits)) => (-1, digits),
        _ => return Err(wrong()),
    };
    if digits.len() != 4 || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(wrong());
    }
    let (hours, minutes) = digits.split_at(2);
    let hours: i32 = hours.parse().map_err(|_| wrong())?;
    let minutes: i32 = minutes.parse().map_err(|_| wrong())?;
    Ok((seconds, sign * (hours * 60 + minutes)))
}

/// The trailers of `%(trailers:only,unfold,separator=%x1d,key_value_separator=%x1c)`; empty when
/// none.
fn trailer_list(trailers: &[u8]) -> Result<Vec<Trailer>, String> {
    if trailers.is_empty() {
        return Ok(Vec::new());
    }
    trailers
        .split(|byte| *byte == TRAILERS)
        .map(|trailer| {
            let mut parts = trailer.splitn(2, |byte| *byte == KEY_VALUE);
            match (parts.next(), parts.next()) {
                (Some(key), Some(value)) => Ok(Trailer {
                    key: text(key),
                    value: text(value),
                }),
                _ => Err(format!("the trailer `{}` has no value", text(trailer))),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::{Person, Trailer};
    use crate::status::Change;

    const ID_A: &str = "9ba7ef97af7f1dd1a92bc20e28dfab34affc423d";
    const ID_B: &str = "634bad3d5d95b082ad0e0f78caab480ddfbf384f";
    const ID_C: &str = "47195abac1c7fbd0f2e0135d8783b27df513adeb";

    /// One commit as git writes it: the format, its NUL, a newline, then the name-status fields.
    fn record(id: &str, date: &str, subject: &str, trailers: &str, status: &[&str]) -> Vec<u8> {
        let mut out = format!(
            "\x1e{id}\x1fSynthetic\x1fsynthetic@example.invalid\x1f{date}\x1f{subject}\x1f{trailers}\0\n"
        )
        .into_bytes();
        for field in status {
            out.extend_from_slice(field.as_bytes());
            out.push(0);
        }
        out
    }

    fn author() -> Person {
        Person {
            name: "Synthetic".to_owned(),
            email: "synthetic@example.invalid".to_owned(),
        }
    }

    fn trailer(key: &str, value: &str) -> Trailer {
        Trailer {
            key: key.to_owned(),
            value: value.to_owned(),
        }
    }

    #[test]
    fn an_empty_answer_has_no_commits() {
        assert_eq!(commits(b""), Ok(Vec::new()));
    }

    #[test]
    fn a_rename_with_a_trailer_gives_both_paths_and_the_trailer() {
        let out = record(
            ID_A,
            "1795790482 +0100",
            "rename ilse",
            "Approved-by\x1cJann",
            &["R083", "mother-ilse.md", "mother-ilsa.md"],
        );
        assert_eq!(
            commits(&out),
            Ok(vec![Commit {
                id: ID_A.to_owned(),
                author: author(),
                time: 1_795_790_482,
                offset_minutes: 60,
                subject: "rename ilse".to_owned(),
                trailers: vec![trailer("Approved-by", "Jann")],
                change: Change::Renamed,
                path: b"mother-ilsa.md".to_vec(),
                from: Some(b"mother-ilse.md".to_vec()),
            }])
        );
    }

    /// Id, time, change, path and path before of a commit.
    type Summary<'a> = (&'a str, i64, Change, &'a [u8], Option<&'a [u8]>);

    #[test]
    fn three_commits_come_back_in_git_order() -> Result<(), String> {
        let mut out = record(ID_A, "3 +0000", "rename", "", &["R090", "a.md", "b.md"]);
        out.extend(record(ID_B, "2 +0000", "edit", "", &["M", "a.md"]));
        out.extend(record(ID_C, "1 +0000", "add", "", &["A", "a.md"]));
        let read = commits(&out)?;
        let summary: Vec<Summary<'_>> = read
            .iter()
            .map(|commit| {
                (
                    commit.id.as_str(),
                    commit.time,
                    commit.change,
                    commit.path.as_slice(),
                    commit.from.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                (ID_A, 3, Change::Renamed, &b"b.md"[..], Some(&b"a.md"[..])),
                (ID_B, 2, Change::Modified, &b"a.md"[..], None),
                (ID_C, 1, Change::Added, &b"a.md"[..], None),
            ]
        );
        assert!(read.iter().all(|commit| commit.trailers.is_empty()));
        Ok(())
    }

    #[test]
    fn a_subject_keeps_percent_quotes_tabs_and_non_ascii() -> Result<(), String> {
        let subject = "fix 100% \"quoted\"\tand 'single' \u{e9}t\u{e9}";
        let out = record(ID_A, "1 +0000", subject, "", &["M", "a.md"]);
        let read = commits(&out)?;
        assert_eq!(read.len(), 1);
        assert_eq!(read[0].subject, subject);
        Ok(())
    }

    #[test]
    fn two_trailers_one_unfolded_come_back_in_order() -> Result<(), String> {
        // git joins the folded second line of the first trailer with a space (`unfold`).
        let out = record(
            ID_A,
            "1 +0000",
            "edit",
            "Approved-by\x1cJann after a long review\x1dCo-Authored-By\x1cClaude <noreply@example.invalid>",
            &["M", "a.md"],
        );
        let read = commits(&out)?;
        assert_eq!(
            read[0].trailers,
            [
                trailer("Approved-by", "Jann after a long review"),
                trailer("Co-Authored-By", "Claude <noreply@example.invalid>"),
            ]
        );
        Ok(())
    }

    #[test]
    fn a_negative_half_hour_zone_gives_negative_minutes() -> Result<(), String> {
        let out = record(ID_A, "1795790482 -0330", "edit", "", &["M", "a.md"]);
        let read = commits(&out)?;
        assert_eq!(read[0].offset_minutes, -210);
        assert_eq!(read[0].time, 1_795_790_482);
        Ok(())
    }

    #[test]
    fn a_path_with_separator_bytes_is_read_whole() -> Result<(), String> {
        let out = record(ID_A, "1 +0000", "edit", "", &["M", "odd\x1e\x1fname.md"]);
        let read = commits(&out)?;
        assert_eq!(read[0].path, b"odd\x1e\x1fname.md");
        Ok(())
    }

    #[test]
    fn a_commit_with_the_wrong_number_of_fields_is_an_error() {
        let out =
            b"\x1eabc\x1fSynthetic\x1fsynthetic@example.invalid\x1f1 +0000\x1fsubject\0\nM\0a.md\0";
        assert!(commits(out).is_err());
    }

    #[test]
    fn a_subject_with_a_unit_separator_is_an_error_not_a_guess() {
        let out = record(ID_A, "1 +0000", "a\x1fb", "", &["M", "a.md"]);
        assert!(commits(&out).is_err());
    }

    #[test]
    fn a_commit_without_its_change_is_an_error() {
        let out = record(ID_A, "1 +0000", "edit", "", &[]);
        assert!(commits(&out).is_err());
    }

    #[test]
    fn a_rename_without_its_second_path_is_an_error() {
        let out = record(ID_A, "1 +0000", "edit", "", &["R083", "a.md"]);
        assert!(commits(&out).is_err());
    }

    #[test]
    fn an_answer_that_does_not_start_a_commit_is_an_error() {
        assert!(commits(b"M\0a.md\0").is_err());
    }

    #[test]
    fn a_date_that_is_not_raw_is_an_error() {
        let out = record(ID_A, "yesterday", "edit", "", &["M", "a.md"]);
        assert!(commits(&out).is_err());
    }
}
