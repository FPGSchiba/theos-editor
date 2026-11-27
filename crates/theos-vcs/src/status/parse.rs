//! Reader for git's documented `status --porcelain=v2 -z --branch` format.
//!
//! Every line ends with a NUL; a `2` line is followed by one more NUL-terminated field, the path
//! the rename or copy came from. See `git help status`, "Porcelain Format Version 2".

use super::{Branch, Change, Conflict, Entry, Status, Upstream};
use crate::staged::parse::fields;

/// Reads the whole answer of `status --porcelain=v2 -z --branch`.
pub(crate) fn status(out: &[u8]) -> Result<Status, String> {
    let mut headers = Headers::default();
    let mut entries = Vec::new();
    let mut lines = fields(out)?.into_iter();
    while let Some(line) = lines.next() {
        match line {
            [b'#', b' ', header @ ..] => headers.read(header)?,
            [b'1', b' ', rest @ ..] => entries.push(ordinary(rest)?),
            [b'2', b' ', rest @ ..] => {
                let from = lines.next().ok_or("a rename has no path it came from")?;
                entries.push(renamed(rest, from)?);
            }
            [b'u', b' ', rest @ ..] => entries.push(unmerged(rest)?),
            [b'?', b' ', path @ ..] => entries.push(Entry::Untracked {
                path: path.to_vec(),
            }),
            other => {
                return Err(format!(
                    "`{}` is not a status line",
                    String::from_utf8_lossy(other)
                ));
            }
        }
    }
    Ok(Status {
        branch: headers.branch()?,
        entries,
    })
}

/// The `# branch.*` headers read so far; other headers (such as `# stash`) are skipped.
#[derive(Default)]
struct Headers {
    oid: Option<String>,
    head: Option<String>,
    upstream: Option<String>,
    ahead_behind: Option<(u32, u32)>,
}

impl Headers {
    /// Reads one header, without its leading `# `.
    fn read(&mut self, header: &[u8]) -> Result<(), String> {
        let header = std::str::from_utf8(header).map_err(|_| "a header is not UTF-8")?;
        let Some((name, value)) = header.split_once(' ') else {
            return Err(format!("`# {header}` has no value"));
        };
        let value = value.to_owned();
        match name {
            "branch.oid" => self.oid = Some(value),
            "branch.head" => self.head = Some(value),
            "branch.upstream" => self.upstream = Some(value),
            "branch.ab" => self.ahead_behind = Some(ahead_behind(&value)?),
            _ => {}
        }
        Ok(())
    }

    /// The branch the headers describe; `--branch` always gives `branch.oid` and `branch.head`.
    ///
    /// An upstream without `branch.ab` is gone (deleted on the remote, or never fetched): there
    /// is nothing to be ahead or behind of, so it reads as no upstream.
    fn branch(self) -> Result<Branch, String> {
        let oid = self.oid.ok_or("the answer has no `# branch.oid`")?;
        let head = self.head.ok_or("the answer has no `# branch.head`")?;
        let upstream = match (self.upstream, self.ahead_behind) {
            (Some(name), Some((ahead, behind))) => Some(Upstream {
                name,
                ahead,
                behind,
            }),
            _ => None,
        };
        Ok(Branch {
            head: (head != "(detached)").then_some(head),
            commit: (oid != "(initial)").then_some(oid),
            upstream,
        })
    }
}

/// Reads `+<ahead> -<behind>`.
fn ahead_behind(value: &str) -> Result<(u32, u32), String> {
    let wrong = || format!("`{value}` is not `+<ahead> -<behind>`");
    let (ahead, behind) = value.split_once(' ').ok_or_else(wrong)?;
    let ahead = ahead.strip_prefix('+').ok_or_else(wrong)?;
    let behind = behind.strip_prefix('-').ok_or_else(wrong)?;
    Ok((
        ahead.parse().map_err(|_| wrong())?,
        behind.parse().map_err(|_| wrong())?,
    ))
}

/// Splits a line after its kind into `count` space-separated fields; the last one, the path,
/// may hold spaces.
fn split(rest: &[u8], count: usize) -> Result<Vec<&[u8]>, String> {
    let parts: Vec<&[u8]> = rest.splitn(count, |byte| *byte == b' ').collect();
    if parts.len() == count {
        Ok(parts)
    } else {
        Err(format!(
            "`{}` has {} fields, not {count}",
            String::from_utf8_lossy(rest),
            parts.len()
        ))
    }
}

/// One side of an `XY` field.
fn change(letter: u8) -> Result<Change, String> {
    Ok(match letter {
        b'.' => Change::Unchanged,
        b'M' => Change::Modified,
        b'T' => Change::TypeChanged,
        b'A' => Change::Added,
        b'D' => Change::Deleted,
        b'R' => Change::Renamed,
        b'C' => Change::Copied,
        other => return Err(format!("`{}` is not a change letter", char::from(other))),
    })
}

/// The staged and unstaged change of an `XY` field.
fn changes(xy: &[u8]) -> Result<(Change, Change), String> {
    match xy {
        [staged, unstaged] => Ok((change(*staged)?, change(*unstaged)?)),
        other => Err(format!(
            "`{}` is not an XY field",
            String::from_utf8_lossy(other)
        )),
    }
}

/// `1 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <path>`.
fn ordinary(rest: &[u8]) -> Result<Entry, String> {
    let parts = split(rest, 8)?;
    let (staged, unstaged) = changes(parts[0])?;
    Ok(Entry::Changed {
        path: parts[7].to_vec(),
        staged,
        unstaged,
    })
}

/// `2 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <X><score> <path>`, then `<origPath>`.
fn renamed(rest: &[u8], from: &[u8]) -> Result<Entry, String> {
    let parts = split(rest, 9)?;
    let (staged, unstaged) = changes(parts[0])?;
    let score = match parts[7] {
        [b'R' | b'C', digits @ ..] => std::str::from_utf8(digits)
            .ok()
            .and_then(|digits| digits.parse::<u8>().ok())
            .filter(|score| *score <= 100),
        _ => None,
    }
    .ok_or_else(|| {
        format!(
            "`{}` is not a rename score",
            String::from_utf8_lossy(parts[7])
        )
    })?;
    Ok(Entry::Renamed {
        from: from.to_vec(),
        to: parts[8].to_vec(),
        staged,
        unstaged,
        score,
    })
}

/// `u <XY> <sub> <m1> <m2> <m3> <mW> <h1> <h2> <h3> <path>`.
fn unmerged(rest: &[u8]) -> Result<Entry, String> {
    let parts = split(rest, 10)?;
    let conflict = match parts[0] {
        b"DD" => Conflict::BothDeleted,
        b"AU" => Conflict::AddedByUs,
        b"UD" => Conflict::DeletedByThem,
        b"UA" => Conflict::AddedByThem,
        b"DU" => Conflict::DeletedByUs,
        b"AA" => Conflict::BothAdded,
        b"UU" => Conflict::BothModified,
        other => {
            return Err(format!(
                "`{}` is not a conflict",
                String::from_utf8_lossy(other)
            ));
        }
    };
    Ok(Entry::Conflicted {
        path: parts[9].to_vec(),
        conflict,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const OID: &str = "3f17a97ec9cef9b0a12d0f2652a74e71364de4d7";
    const ZERO: &str = "0000000000000000000000000000000000000000";

    /// The two headers `--branch` always gives, for `main` at `OID`.
    fn main_headers() -> Vec<u8> {
        format!("# branch.oid {OID}\0# branch.head main\0").into_bytes()
    }

    /// `main_headers` followed by `lines`.
    fn on_main(lines: &[u8]) -> Vec<u8> {
        let mut out = main_headers();
        out.extend_from_slice(lines);
        out
    }

    fn entries(lines: &[u8]) -> Result<Vec<Entry>, String> {
        Ok(status(&on_main(lines))?.entries)
    }

    fn line_1(xy: &str, path: &[u8]) -> Vec<u8> {
        let mut line = format!("1 {xy} N... 100644 100644 100644 {OID} {OID} ").into_bytes();
        line.extend_from_slice(path);
        line.push(0);
        line
    }

    #[test]
    fn branch_headers_with_an_upstream() -> Result<(), String> {
        let out = format!(
            "# branch.oid {OID}\0# branch.head main\0# branch.upstream origin/main\0\
             # branch.ab +1 -2\0"
        );
        assert_eq!(
            status(out.as_bytes())?,
            Status {
                branch: Branch {
                    head: Some("main".to_owned()),
                    commit: Some(OID.to_owned()),
                    upstream: Some(Upstream {
                        name: "origin/main".to_owned(),
                        ahead: 1,
                        behind: 2,
                    }),
                },
                entries: Vec::new(),
            }
        );
        Ok(())
    }

    #[test]
    fn branch_headers_without_an_upstream() -> Result<(), String> {
        let branch = status(&main_headers())?.branch;
        assert_eq!(
            branch,
            Branch {
                head: Some("main".to_owned()),
                commit: Some(OID.to_owned()),
                upstream: None,
            }
        );
        Ok(())
    }

    #[test]
    fn an_upstream_that_is_gone_reads_as_none() -> Result<(), String> {
        let out = format!("# branch.oid {OID}\0# branch.head main\0# branch.upstream origin/x\0");
        assert_eq!(status(out.as_bytes())?.branch.upstream, None);
        Ok(())
    }

    #[test]
    fn initial_has_no_commit_and_detached_has_no_head() -> Result<(), String> {
        let initial = status(b"# branch.oid (initial)\0# branch.head main\0")?.branch;
        assert_eq!(initial.commit, None);
        assert_eq!(initial.head, Some("main".to_owned()));
        let detached =
            status(format!("# branch.oid {OID}\0# branch.head (detached)\0").as_bytes())?.branch;
        assert_eq!(detached.head, None);
        assert_eq!(detached.commit, Some(OID.to_owned()));
        Ok(())
    }

    #[test]
    fn other_headers_are_skipped() -> Result<(), String> {
        let out = on_main(b"# stash 3\0");
        assert_eq!(status(&out)?.entries, Vec::new());
        Ok(())
    }

    #[test]
    fn ordinary_lines_give_both_changes() -> Result<(), String> {
        let mut lines = line_1("M.", b"a.md");
        lines.extend(line_1(".M", b"b.md"));
        lines.extend(line_1("MM", b"c.md"));
        lines.extend(line_1("AD", b"d.md"));
        lines.extend(line_1(".T", b"e.md"));
        lines.extend(line_1(".A", b"f.md"));
        let changed = |path: &[u8], staged, unstaged| Entry::Changed {
            path: path.to_vec(),
            staged,
            unstaged,
        };
        assert_eq!(
            entries(&lines)?,
            vec![
                changed(b"a.md", Change::Modified, Change::Unchanged),
                changed(b"b.md", Change::Unchanged, Change::Modified),
                changed(b"c.md", Change::Modified, Change::Modified),
                changed(b"d.md", Change::Added, Change::Deleted),
                changed(b"e.md", Change::Unchanged, Change::TypeChanged),
                changed(b"f.md", Change::Unchanged, Change::Added),
            ]
        );
        Ok(())
    }

    #[test]
    fn renames_and_copies_in_the_index_and_in_the_work_tree() -> Result<(), String> {
        let lines = format!(
            "2 R. N... 100644 100644 100644 {OID} {OID} R100 lore/new.md\0lore/old.md\0\
             2 RM N... 100644 100644 100644 {OID} {OID} R92 b.md\0a.md\0\
             2 C. N... 100644 100644 100644 {OID} {OID} C75 copy.md\0source.md\0\
             2 .R N... 100644 100644 100644 {OID} {OID} R84 mother-ilsa.md\0mother-ilse.md\0"
        );
        let renamed = |from: &[u8], to: &[u8], staged, unstaged, score| Entry::Renamed {
            from: from.to_vec(),
            to: to.to_vec(),
            staged,
            unstaged,
            score,
        };
        assert_eq!(
            entries(lines.as_bytes())?,
            vec![
                renamed(
                    b"lore/old.md",
                    b"lore/new.md",
                    Change::Renamed,
                    Change::Unchanged,
                    100
                ),
                renamed(b"a.md", b"b.md", Change::Renamed, Change::Modified, 92),
                renamed(
                    b"source.md",
                    b"copy.md",
                    Change::Copied,
                    Change::Unchanged,
                    75
                ),
                renamed(
                    b"mother-ilse.md",
                    b"mother-ilsa.md",
                    Change::Unchanged,
                    Change::Renamed,
                    84
                ),
            ]
        );
        Ok(())
    }

    #[test]
    fn unmerged_lines_give_each_conflict() -> Result<(), String> {
        let kinds = [
            ("DD", Conflict::BothDeleted),
            ("AU", Conflict::AddedByUs),
            ("UD", Conflict::DeletedByThem),
            ("UA", Conflict::AddedByThem),
            ("DU", Conflict::DeletedByUs),
            ("AA", Conflict::BothAdded),
            ("UU", Conflict::BothModified),
        ];
        for (xy, conflict) in kinds {
            let line =
                format!("u {xy} N... 100644 100644 100644 100644 {OID} {OID} {OID} c d.md\0");
            assert_eq!(
                entries(line.as_bytes())?,
                vec![Entry::Conflicted {
                    path: b"c d.md".to_vec(),
                    conflict,
                }],
                "{xy}"
            );
        }
        Ok(())
    }

    #[test]
    fn untracked_lines_give_their_path() -> Result<(), String> {
        assert_eq!(
            entries(b"? new.md\0? folder/x.md\0")?,
            vec![
                Entry::Untracked {
                    path: b"new.md".to_vec()
                },
                Entry::Untracked {
                    path: b"folder/x.md".to_vec()
                },
            ]
        );
        Ok(())
    }

    #[test]
    fn a_path_with_spaces_and_non_ascii_bytes_is_kept_as_bytes() -> Result<(), String> {
        let path = "lore/a b [x] é.md".as_bytes();
        let mut lines = line_1(".M", path);
        lines.extend(format!("2 R. N... 100644 100644 100644 {ZERO} {ZERO} R100 ").bytes());
        lines.extend_from_slice(path);
        lines.extend_from_slice(b"\0old name.md\0? ");
        lines.extend_from_slice(path);
        lines.push(0);
        assert_eq!(
            entries(&lines)?,
            vec![
                Entry::Changed {
                    path: path.to_vec(),
                    staged: Change::Unchanged,
                    unstaged: Change::Modified,
                },
                Entry::Renamed {
                    from: b"old name.md".to_vec(),
                    to: path.to_vec(),
                    staged: Change::Renamed,
                    unstaged: Change::Unchanged,
                    score: 100,
                },
                Entry::Untracked {
                    path: path.to_vec()
                },
            ]
        );
        Ok(())
    }

    #[test]
    fn truncated_or_malformed_output_is_refused() {
        let wrong: [&[u8]; 12] = [
            b"1 .M N... 100644\0",
            b"1 .M N... 100644 100644 100644 aaaa bbbb a.md",
            b"2 .R N... 100644 100644 100644 aaaa bbbb R84 new.md\0",
            b"2 .R N... 100644 100644 100644 aaaa bbbb X84 new.md\0old.md\0",
            b"2 .R N... 100644 100644 100644 aaaa bbbb R184 new.md\0old.md\0",
            b"u XY N... 100644 100644 100644 100644 a b c p\0",
            b"u UU N... 100644 100644 100644 100644 a b\0",
            b"1 .Q N... 100644 100644 100644 aaaa bbbb a.md\0",
            b"1 .MM N... 100644 100644 100644 aaaa bbbb a.md\0",
            b"! ignored.md\0",
            b"# branch.ab 1 2\0",
            b"#\0",
        ];
        for lines in wrong {
            assert!(
                status(&on_main(lines)).is_err(),
                "{}",
                String::from_utf8_lossy(lines)
            );
        }
    }

    #[test]
    fn missing_branch_headers_are_refused() {
        assert!(status(b"").is_err());
        assert!(status(b"# branch.head main\0").is_err());
        assert!(status(format!("# branch.oid {OID}\0").as_bytes()).is_err());
    }
}
