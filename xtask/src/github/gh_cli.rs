//! The `gh` command line as the GitHub client: every call is `gh api <endpoint>`.
//!
//! `gh` is started with an argument list, never through a shell, with no stdin, and takes its
//! token from `GH_TOKEN` in the environment it inherits. The adapter is thin on purpose: a call is
//! described as a `Request` (which the tests read without running anything), `gh` runs it, and
//! the answer is read as JSON. What a pull request wrote never becomes part of an error: a failed
//! call is reported as `METHOD endpoint: HTTP <status>`, or the exit code of `gh` when it said
//! no status. The status is read from `gh`'s stderr as at most three digits, and the rest of
//! stderr is dropped, because it can quote the request.

use std::ffi::OsStr;
use std::process::{Command, Stdio};

use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::{Comment, LabelSpec, PullRequests, failed};
use crate::Result;

/// The entries GitHub is asked for on each page of a list; its largest.
const PAGE_SIZE: &str = "per_page=100";

/// GitHub through the `gh` command line.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GhCli {
    /// The repository as `owner/name`, from `GITHUB_REPOSITORY`.
    pub repo: String,
}

/// One call of `gh api`.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Request {
    method: &'static str,
    endpoint: String,
    /// Read every page of the answer.
    paginate: bool,
    /// Parameters as `key=value`, sent as strings: in the query of a GET, in the body otherwise.
    fields: Vec<String>,
}

impl Request {
    fn get(endpoint: String) -> Self {
        Self {
            method: "GET",
            endpoint,
            paginate: false,
            fields: Vec::new(),
        }
    }

    /// A GET of a list: all pages, as many entries per page as GitHub gives.
    fn list(endpoint: String) -> Self {
        Self {
            paginate: true,
            fields: vec![PAGE_SIZE.to_owned()],
            ..Self::get(endpoint)
        }
    }

    fn post(endpoint: String, fields: Vec<String>) -> Self {
        Self {
            method: "POST",
            fields,
            ..Self::get(endpoint)
        }
    }

    fn delete(endpoint: String) -> Self {
        Self {
            method: "DELETE",
            ..Self::get(endpoint)
        }
    }

    /// The arguments after `gh`. Fields go through `-f`, which sends the value as it is: `-F`
    /// would read a value that starts with `@` as a file name.
    fn args(&self) -> Vec<String> {
        let mut args = vec![
            "api".to_owned(),
            "--method".to_owned(),
            self.method.to_owned(),
        ];
        if self.paginate {
            args.push("--paginate".to_owned());
        }
        for field in &self.fields {
            args.push("-f".to_owned());
            args.push(field.clone());
        }
        args.push(self.endpoint.clone());
        args
    }

    /// `METHOD endpoint`, which is how errors name the call.
    fn describe(&self) -> String {
        format!("{} {}", self.method, self.endpoint)
    }
}

impl GhCli {
    fn issue(&self, pr: u64, part: &str) -> String {
        format!("repos/{}/issues/{pr}/{part}", self.repo)
    }
}

impl PullRequests for GhCli {
    fn labels(&self, pr: u64) -> Result<Vec<String>> {
        let request = Request::list(self.issue(pr, "labels"));
        label_names(&request.describe(), &call(&request)?)
    }

    fn add_label(&self, pr: u64, name: &str) -> Result<()> {
        let request = Request::post(self.issue(pr, "labels"), vec![format!("labels[]={name}")]);
        call(&request).map(drop)
    }

    fn remove_label(&self, pr: u64, name: &str) -> Result<()> {
        let request = Request::delete(self.issue(pr, &format!("labels/{}", encode(name))));
        call(&request).map(drop)
    }

    fn ensure_label(&self, spec: &LabelSpec) -> Result<()> {
        let list = Request::list(format!("repos/{}/labels", self.repo));
        let existing = label_names(&list.describe(), &call(&list)?)?;
        match create_label(&self.repo, &existing, spec) {
            Some(create) => call(&create).map(drop),
            None => Ok(()),
        }
    }

    fn comments(&self, pr: u64) -> Result<Vec<Comment>> {
        let request = Request::list(self.issue(pr, "comments"));
        comments_from(&request.describe(), &call(&request)?)
    }

    fn comment(&self, pr: u64, body: &str) -> Result<()> {
        let request = Request::post(self.issue(pr, "comments"), vec![format!("body={body}")]);
        call(&request).map(drop)
    }
}

/// The request that creates the label `spec`, or `None` when the repository has it already
/// (`existing` are the names it has). GitHub keeps label names unique whatever their case.
fn create_label(repo: &str, existing: &[String], spec: &LabelSpec) -> Option<Request> {
    if existing
        .iter()
        .any(|name| name.eq_ignore_ascii_case(&spec.name))
    {
        return None;
    }
    Some(Request::post(
        format!("repos/{repo}/labels"),
        vec![
            format!("name={}", spec.name),
            format!("color={}", spec.color),
            format!("description={}", spec.description),
        ],
    ))
}

/// Runs `gh` with the request and returns what it wrote to stdout.
fn call(request: &Request) -> Result<String> {
    run_program(OsStr::new("gh"), &request.args(), &request.describe())
}

/// Runs `program` with `args` and returns its stdout. `request` names the call in errors.
fn run_program(program: &OsStr, args: &[String], request: &str) -> Result<String> {
    tracing::debug!(request, "gh");
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .env("GH_PROMPT_DISABLED", "1")
        .output()
        .map_err(|source| failed(request, &format!("cannot start `gh` ({})", source.kind())))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let what = match (http_status(&stderr), output.status.code()) {
        (Some(status), _) => format!("HTTP {status}"),
        (None, Some(code)) => format!("`gh` exited with status {code}"),
        (None, None) => "`gh` was ended by a signal".to_owned(),
    };
    Err(failed(request, &what))
}

/// The HTTP status in the stderr of a failed `gh api`, which ends in `(HTTP 404)` or the like.
/// Only a number from 100 to 599 comes out; nothing else of the text does.
fn http_status(stderr: &str) -> Option<u16> {
    let (_, rest) = stderr.split_once("(HTTP ")?;
    let digits: String = rest
        .chars()
        .take_while(char::is_ascii_digit)
        .take(3)
        .collect();
    digits
        .parse()
        .ok()
        .filter(|status| (100..=599).contains(status))
}

/// `segment` as one part of a URL path: everything but letters, digits and `-._~` is `%XX`.
fn encode(segment: &str) -> String {
    use std::fmt::Write as _;
    segment.bytes().fold(String::new(), |mut encoded, byte| {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            // Writing to a `String` cannot fail.
            let _ = write!(encoded, "%{byte:02X}");
        }
        encoded
    })
}

#[derive(Deserialize)]
struct LabelItem {
    name: String,
}

#[derive(Deserialize)]
struct CommentItem {
    user: Option<UserItem>,
    body: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Deserialize)]
struct UserItem {
    login: String,
}

/// The names in the labels `json` lists.
fn label_names(request: &str, json: &str) -> Result<Vec<String>> {
    let labels: Vec<LabelItem> = pages(request, json)?;
    Ok(labels.into_iter().map(|label| label.name).collect())
}

/// The comments `json` lists. A comment of a deleted account has no author, which no check
/// trusts: it gets the empty string.
fn comments_from(request: &str, json: &str) -> Result<Vec<Comment>> {
    let comments: Vec<CommentItem> = pages(request, json)?;
    Ok(comments
        .into_iter()
        .map(|comment| Comment {
            author: comment.user.map(|user| user.login).unwrap_or_default(),
            body: comment.body.unwrap_or_default(),
            created_at: comment.created_at,
            updated_at: comment.updated_at,
        })
        .collect())
}

/// The entries of every array in `json`. `gh api --paginate` joins the pages of a list into one
/// array, and older versions wrote one array per page, one after the other; reading arrays until
/// the text ends takes both.
fn pages<T: DeserializeOwned>(request: &str, json: &str) -> Result<Vec<T>> {
    let mut entries = Vec::new();
    for page in serde_json::Deserializer::from_str(json).into_iter::<Vec<T>>() {
        // The parser's message can quote what it read, so it is not passed on.
        let page =
            page.map_err(|_| failed(request, "the answer is not a list of the expected shape"))?;
        entries.extend(page);
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    const REPO: &str = "owner/repo";

    fn gh() -> GhCli {
        GhCli {
            repo: REPO.to_owned(),
        }
    }

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| (*arg).to_owned()).collect()
    }

    fn spec() -> LabelSpec {
        LabelSpec {
            name: "needs-jann".to_owned(),
            color: "d93f0b".to_owned(),
            description: "Waits for Jann".to_owned(),
        }
    }

    #[test]
    fn a_list_reads_every_page_with_the_largest_page_size() {
        let request = Request::list(gh().issue(7, "labels"));
        assert_eq!(request.describe(), "GET repos/owner/repo/issues/7/labels");
        assert_eq!(
            request.args(),
            strings(&[
                "api",
                "--method",
                "GET",
                "--paginate",
                "-f",
                "per_page=100",
                "repos/owner/repo/issues/7/labels"
            ])
        );
    }

    #[test]
    fn a_post_sends_its_fields_as_strings_and_a_delete_has_none() {
        let post = Request::post(
            gh().issue(7, "comments"),
            vec!["body=@file <!-- x -->".to_owned()],
        );
        assert_eq!(
            post.args(),
            strings(&[
                "api",
                "--method",
                "POST",
                "-f",
                "body=@file <!-- x -->",
                "repos/owner/repo/issues/7/comments"
            ])
        );
        let delete = Request::delete(gh().issue(7, "labels/approved"));
        assert_eq!(
            delete.args(),
            strings(&[
                "api",
                "--method",
                "DELETE",
                "repos/owner/repo/issues/7/labels/approved"
            ])
        );
        assert_eq!(
            delete.describe(),
            "DELETE repos/owner/repo/issues/7/labels/approved"
        );
    }

    #[test]
    fn a_label_is_created_unless_the_repository_has_it_in_any_case() {
        let existing = strings(&["bug", "Needs-Jann"]);
        assert_eq!(create_label(REPO, &existing, &spec()), None);
        let request = create_label(REPO, &strings(&["bug"]), &spec());
        assert_eq!(
            request.map(|request| request.args()),
            Some(strings(&[
                "api",
                "--method",
                "POST",
                "-f",
                "name=needs-jann",
                "-f",
                "color=d93f0b",
                "-f",
                "description=Waits for Jann",
                "repos/owner/repo/labels"
            ]))
        );
    }

    #[test]
    fn path_segments_are_percent_encoded() {
        assert_eq!(encode("approved"), "approved");
        assert_eq!(encode("a-b_c.d~e"), "a-b_c.d~e");
        assert_eq!(encode("needs jann/é?"), "needs%20jann%2F%C3%A9%3F");
    }

    #[test]
    fn labels_are_read_from_one_page_or_many() -> crate::Result<()> {
        let request = "GET labels";
        assert_eq!(
            label_names(request, r#"[{"name":"a","id":1},{"name":"b"}]"#)?,
            ["a", "b"]
        );
        // Older `gh` versions write the pages one after the other.
        assert_eq!(
            label_names(request, "[{\"name\":\"a\"}]\n[{\"name\":\"b\"}][]")?,
            ["a", "b"]
        );
        assert_eq!(label_names(request, "[]")?, Vec::<String>::new());
        assert_eq!(label_names(request, "")?, Vec::<String>::new());
        Ok(())
    }

    #[test]
    fn comments_carry_the_login_the_text_and_both_times() -> crate::Result<()> {
        let json = r#"[
            {"id":1,"user":{"login":"github-actions[bot]","id":41898282},"body":"marker",
             "created_at":"2026-10-31T10:00:00Z","updated_at":"2026-10-31T10:00:00Z"},
            {"id":2,"user":null,"body":null,
             "created_at":"2026-10-31T11:00:00Z","updated_at":"2026-10-31T12:00:00Z"}
        ][{"id":3,"user":{"login":"jann"},"body":"later",
           "created_at":"2026-11-01T10:00:00Z","updated_at":"2026-11-01T10:00:00Z"}]"#;
        let comments = comments_from("GET comments", json)?;
        assert_eq!(
            comments,
            [
                Comment {
                    author: "github-actions[bot]".to_owned(),
                    body: "marker".to_owned(),
                    created_at: "2026-10-31T10:00:00Z".to_owned(),
                    updated_at: "2026-10-31T10:00:00Z".to_owned(),
                },
                Comment {
                    author: String::new(),
                    body: String::new(),
                    created_at: "2026-10-31T11:00:00Z".to_owned(),
                    updated_at: "2026-10-31T12:00:00Z".to_owned(),
                },
                Comment {
                    author: "jann".to_owned(),
                    body: "later".to_owned(),
                    created_at: "2026-11-01T10:00:00Z".to_owned(),
                    updated_at: "2026-11-01T10:00:00Z".to_owned(),
                },
            ]
        );
        Ok(())
    }

    #[test]
    fn an_answer_of_the_wrong_shape_is_an_error_that_quotes_nothing() {
        for json in [
            r#"{"message":"SECRET text from a comment"}"#,
            r#"[{"name":"SECRET text"}"#,
            r#"[{"login":"SECRET text"}]"#,
            r#"[{"name":7}]"#,
        ] {
            let result = label_names("GET labels", json);
            let Err(Error::GitHub { message }) = &result else {
                panic!("expected a GitHub error for {json}, got {result:?}");
            };
            assert!(message.starts_with("GET labels: "), "{message}");
            assert!(!message.contains("SECRET"), "{message}");
        }
    }

    #[test]
    fn the_status_is_the_number_after_http_and_nothing_else() {
        assert_eq!(http_status("gh: Not Found (HTTP 404)\n"), Some(404));
        assert_eq!(http_status("gh: Validation Failed (HTTP 422)"), Some(422));
        assert_eq!(http_status("gh: no status here"), None);
        assert_eq!(http_status("(HTTP )"), None);
        assert_eq!(http_status("(HTTP 99)"), None);
        assert_eq!(http_status("(HTTP 600)"), None);
        // Longer numbers are cut at three digits: 4040 reads as 404.
        assert_eq!(http_status("(HTTP 4040)"), Some(404));
    }

    #[test]
    fn what_gh_prints_comes_back_when_it_succeeds() -> crate::Result<()> {
        let output = run_program(OsStr::new("git"), &strings(&["--version"]), "GET x")?;
        assert!(output.starts_with("git version"), "{output}");
        Ok(())
    }

    #[test]
    fn a_failed_call_names_the_request_and_the_exit_code_only() {
        // `git check-ref-format` fails with exit code 1 and says nothing a test could rely on.
        let args = strings(&["check-ref-format", "refs/heads/.."]);
        let result = run_program(OsStr::new("git"), &args, "GET repos/o/r/issues/7/labels");
        let Err(Error::GitHub { message }) = &result else {
            panic!("expected a GitHub error, got {result:?}");
        };
        assert_eq!(
            message,
            "GET repos/o/r/issues/7/labels: `gh` exited with status 1"
        );
    }

    #[test]
    fn a_missing_gh_is_a_github_error() {
        let result = run_program(
            OsStr::new("xtask-no-such-program"),
            &strings(&["api"]),
            "POST repos/o/r/issues/7/comments",
        );
        let Err(Error::GitHub { message }) = &result else {
            panic!("expected a GitHub error, got {result:?}");
        };
        assert!(
            message.starts_with("POST repos/o/r/issues/7/comments: cannot start `gh` ("),
            "{message}"
        );
    }
}
