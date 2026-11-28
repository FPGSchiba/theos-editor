//! Reads the ref line of `git push --porcelain`: `<flag>\t<from>:<to>\t<summary> (<reason>)`.

use crate::push::Rejection;

/// What git reported for the one ref that was pushed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RefLine {
    /// The remote already had the commit.
    UpToDate,
    /// The ref moved; ids are as abbreviated as git printed them.
    Updated {
        /// The old tip, `None` for a new branch.
        from: Option<String>,
        /// The new tip, `None` when the summary names no id.
        to: Option<String>,
    },
    /// The remote did not take the ref.
    Rejected(Rejection),
}

/// Finds the line whose destination is `refs/heads/<branch>` in porcelain `stdout`.
pub(crate) fn ref_line(stdout: &str, branch: &str) -> Option<RefLine> {
    let wanted = format!("refs/heads/{branch}");
    stdout.lines().find_map(|line| {
        let mut parts = line.splitn(3, '\t');
        let flag = parts.next()?;
        let refs = parts.next()?;
        let summary = parts.next()?;
        (refs.rsplit_once(':')?.1 == wanted).then(|| classify(flag, summary))
    })
}

fn classify(flag: &str, summary: &str) -> RefLine {
    match flag {
        "=" => RefLine::UpToDate,
        "!" => RefLine::Rejected(rejection(summary)),
        _ => {
            let range = summary.split_whitespace().next().unwrap_or_default();
            match range.split_once("..") {
                Some((from, to)) => RefLine::Updated {
                    from: Some(from.to_owned()),
                    to: Some(to.trim_start_matches('.').to_owned()),
                },
                None => RefLine::Updated {
                    from: None,
                    to: None,
                },
            }
        }
    }
}

fn rejection(summary: &str) -> Rejection {
    if summary.starts_with("[remote rejected]") {
        Rejection::RemoteRefused
    } else if summary.ends_with("(non-fast-forward)") || summary.ends_with("(fetch first)") {
        Rejection::NotFastForward
    } else {
        Rejection::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(flag: &str, summary: &str) -> Option<RefLine> {
        let text = format!("To x\n{flag}\tHEAD:refs/heads/main\t{summary}\nDone\n");
        ref_line(&text, "main")
    }

    // AC: CE-22.3
    #[test]
    fn an_up_to_date_branch() {
        assert_eq!(line("=", "[up to date]"), Some(RefLine::UpToDate));
    }

    // AC: CE-22.3
    #[test]
    fn a_new_branch_has_no_old_tip() {
        assert_eq!(
            line("*", "[new branch]"),
            Some(RefLine::Updated {
                from: None,
                to: None
            })
        );
    }

    // AC: CE-22.3
    #[test]
    fn a_range_gives_both_tips() {
        let expected = Some(RefLine::Updated {
            from: Some("abc".into()),
            to: Some("def".into()),
        });
        assert_eq!(line(" ", "abc..def"), expected);
        assert_eq!(line("+", "abc...def (forced update)"), expected);
    }

    // AC: CE-22.3
    #[test]
    fn rejections_are_told_apart() {
        let rejected = |s| line("!", s);
        let some = |r| Some(RefLine::Rejected(r));
        assert_eq!(
            rejected("[rejected] (non-fast-forward)"),
            some(Rejection::NotFastForward)
        );
        assert_eq!(
            rejected("[rejected] (fetch first)"),
            some(Rejection::NotFastForward)
        );
        assert_eq!(
            rejected("[remote rejected] (pre-receive hook declined)"),
            some(Rejection::RemoteRefused)
        );
        assert_eq!(rejected("[rejected] (stale info)"), some(Rejection::Other));
    }

    #[test]
    fn other_refs_are_ignored() {
        let text = "=\tHEAD:refs/heads/dev\t[up to date]\n";
        assert_eq!(ref_line(text, "main"), None);
    }
}
