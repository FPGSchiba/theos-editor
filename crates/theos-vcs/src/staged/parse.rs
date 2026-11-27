//! Readers for the documented formats of `diff -z`, `ls-files -s -z` and `cat-file --batch`.

use super::{StagedPath, Status, Tracked};

/// Splits NUL-terminated output into its fields; the last field must end with a NUL too.
fn fields(out: &[u8]) -> Result<Vec<&[u8]>, String> {
    if out.is_empty() {
        return Ok(Vec::new());
    }
    let body = out
        .strip_suffix(&[0])
        .ok_or("the output does not end with a NUL")?;
    Ok(body.split(|byte| *byte == 0).collect())
}

fn status(letter: u8) -> Status {
    match letter {
        b'A' => Status::Added,
        b'D' => Status::Deleted,
        b'M' => Status::Modified,
        b'T' => Status::TypeChanged,
        b'U' => Status::Unmerged,
        other => Status::Other(other),
    }
}

/// Reads `diff --name-status -z` (no renames): `<letter> NUL <path> NUL` per entry.
pub(super) fn name_status(out: &[u8]) -> Result<Vec<StagedPath>, String> {
    let fields = fields(out)?;
    if fields.len() % 2 != 0 {
        return Err("a status letter has no path".to_owned());
    }
    let (pairs, _) = fields.as_chunks::<2>();
    pairs
        .iter()
        .map(|[letter, path]| match letter {
            [letter] => Ok(StagedPath {
                path: path.to_vec(),
                status: status(*letter),
            }),
            other => Err(format!(
                "`{}` is not a status letter",
                String::from_utf8_lossy(other)
            )),
        })
        .collect()
}

/// Reads `ls-files -s -z`: `<mode> SP <object> SP <stage> TAB <path> NUL`, stage 0 only.
pub(crate) fn ls_files(out: &[u8]) -> Result<Vec<Tracked>, String> {
    let mut entries = Vec::new();
    for field in fields(out)? {
        let tab = field
            .iter()
            .position(|byte| *byte == b'\t')
            .ok_or("an entry has no tab before its path")?;
        let head = std::str::from_utf8(&field[..tab]).map_err(|_| "an entry is not UTF-8")?;
        let mut parts = head.split(' ');
        let (Some(mode), Some(object), Some(stage), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(format!("`{head}` is not `<mode> <object> <stage>`"));
        };
        let mode =
            u32::from_str_radix(mode, 8).map_err(|_| format!("`{mode}` is not an octal mode"))?;
        let stage: u8 = stage
            .parse()
            .map_err(|_| format!("`{stage}` is not a stage"))?;
        // Stages 1 to 3 belong to a merge conflict; the snapshot keeps the settled entries.
        if stage == 0 {
            entries.push(Tracked {
                path: field[tab + 1..].to_vec(),
                mode,
                object: object.to_owned(),
            });
        }
    }
    Ok(entries)
}

/// Reads `cat-file --batch` for `count` objects: `<oid> SP <type> SP <size> LF <bytes> LF`.
pub(super) fn batch(out: &[u8], count: usize) -> Result<Vec<Vec<u8>>, String> {
    let mut rest = out;
    let mut blobs = Vec::with_capacity(count);
    for _ in 0..count {
        let end = rest
            .iter()
            .position(|byte| *byte == b'\n')
            .ok_or("the answer ends inside a header")?;
        let header = String::from_utf8_lossy(&rest[..end]).into_owned();
        let mut parts = header.split(' ');
        let (Some(object), Some(kind)) = (parts.next(), parts.next()) else {
            return Err(format!("`{header}` is not a header"));
        };
        if kind == "missing" {
            return Err(format!("object {object} is missing"));
        }
        let size = parts
            .next()
            .and_then(|size| size.parse::<usize>().ok())
            .ok_or_else(|| format!("`{header}` has no size"))?;
        let body = &rest[end + 1..];
        let blob = body
            .get(..size)
            .ok_or_else(|| format!("object {object} is shorter than its {size} bytes"))?;
        rest = match body[size..].split_first() {
            Some((b'\n', after)) => after,
            _ => return Err(format!("object {object} is not followed by a newline")),
        };
        blobs.push(blob.to_vec());
    }
    if rest.is_empty() {
        Ok(blobs)
    } else {
        Err("the answer holds more objects than were asked for".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn staged(path: &[u8], status: Status) -> StagedPath {
        StagedPath {
            path: path.to_vec(),
            status,
        }
    }

    #[test]
    fn name_status_reads_letters_and_paths() {
        let out = b"A\0lore/a b.md\0M\0lore/new\nline.md\0D\0x\0T\0y\0U\0z\0R\0w\0";
        assert_eq!(
            name_status(out),
            Ok(vec![
                staged(b"lore/a b.md", Status::Added),
                staged(b"lore/new\nline.md", Status::Modified),
                staged(b"x", Status::Deleted),
                staged(b"y", Status::TypeChanged),
                staged(b"z", Status::Unmerged),
                staged(b"w", Status::Other(b'R')),
            ])
        );
        assert_eq!(name_status(b""), Ok(Vec::new()));
    }

    #[test]
    fn name_status_rejects_malformed_output() {
        assert!(name_status(b"A\0").is_err());
        assert!(name_status(b"AM\0p\0").is_err());
        assert!(name_status(b"A\0p").is_err());
    }

    #[test]
    fn ls_files_reads_mode_object_and_path_and_keeps_stage_zero() {
        let out = b"100644 aaaa 0\tlore/a b.md\0\
                    100755 bbbb 0\ttab\there\0\
                    100644 cccc 1\tconflict.md\0\
                    100644 dddd 2\tconflict.md\0\
                    120000 eeee 0\tlink\0";
        assert_eq!(
            ls_files(out),
            Ok(vec![
                Tracked {
                    path: b"lore/a b.md".to_vec(),
                    mode: 0o100_644,
                    object: "aaaa".to_owned(),
                },
                Tracked {
                    path: b"tab\there".to_vec(),
                    mode: 0o100_755,
                    object: "bbbb".to_owned(),
                },
                Tracked {
                    path: b"link".to_vec(),
                    mode: 0o120_000,
                    object: "eeee".to_owned(),
                },
            ])
        );
    }

    #[test]
    fn ls_files_rejects_malformed_output() {
        assert!(ls_files(b"100644 aaaa 0 no-tab\0").is_err());
        assert!(ls_files(b"zzz aaaa 0\tp\0").is_err());
        assert!(ls_files(b"100644 aaaa\tp\0").is_err());
        assert!(ls_files(b"100644 aaaa x\tp\0").is_err());
        assert!(ls_files(b"100644 aaaa 0\tp").is_err());
    }

    #[test]
    fn batch_reads_each_blob() {
        let out = b"aaaa blob 3\nabc\nbbbb blob 0\n\ncccc blob 4\nx\ny\n\n";
        assert_eq!(
            batch(out, 3),
            Ok(vec![b"abc".to_vec(), Vec::new(), b"x\ny\n".to_vec()])
        );
    }

    #[test]
    fn batch_rejects_a_missing_object_and_a_short_blob() {
        assert!(batch(b"aaaa missing\n", 1).is_err());
        assert!(batch(b"aaaa blob 10\nabc\n", 1).is_err());
        assert!(batch(b"aaaa blob 3\nabcX", 1).is_err());
        assert!(batch(b"aaaa blob 99999999999999999999\nabc\n", 1).is_err());
        assert!(batch(b"aaaa blob\nabc\n", 1).is_err());
        assert!(batch(b"aaaa blob 3", 1).is_err());
        assert!(batch(b"aaaa blob 3\nabc\n", 2).is_err());
        assert!(batch(b"aaaa blob 3\nabc\nextra", 1).is_err());
    }
}
