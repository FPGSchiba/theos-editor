//! A GitHub that lives in memory and remembers every call, for tests.
//!
//! The labels and comments change as the real ones would: a label that is added is there for the
//! next read, a comment that is written is by `github-actions[bot]` and was never edited. The
//! calls are kept in order, so a test can say exactly what a check did.

use std::cell::RefCell;

use super::{Comment, LabelSpec, PullRequests, failed};
use crate::Result;

/// What a check asked of GitHub.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Call {
    /// `labels(pr)`.
    Labels(u64),
    /// `add_label(pr, name)`.
    AddLabel(u64, String),
    /// `remove_label(pr, name)`.
    RemoveLabel(u64, String),
    /// `ensure_label(spec)`.
    EnsureLabel(LabelSpec),
    /// `comments(pr)`.
    Comments(u64),
    /// `comment(pr, body)`.
    Comment(u64, String),
}

#[derive(Default)]
struct State {
    labels: Vec<String>,
    comments: Vec<Comment>,
    calls: Vec<Call>,
    comments_fail: bool,
}

/// A recording stand-in for GitHub, with one pull request's labels and comments.
#[derive(Default)]
pub struct FakeGitHub {
    state: RefCell<State>,
}

impl FakeGitHub {
    /// A pull request with no labels and no comments.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The pull request starts with these labels.
    #[must_use]
    pub fn with_labels(self, names: &[&str]) -> Self {
        self.state.borrow_mut().labels = names.iter().map(|name| (*name).to_owned()).collect();
        self
    }

    /// The pull request starts with this comment, after the ones it has.
    #[must_use]
    pub fn with_comment(self, comment: Comment) -> Self {
        self.state.borrow_mut().comments.push(comment);
        self
    }

    /// Writing a comment fails, as it does when GitHub is down.
    #[must_use]
    pub fn with_failing_comments(self) -> Self {
        self.state.borrow_mut().comments_fail = true;
        self
    }

    /// Every call so far, oldest first.
    #[must_use]
    pub fn calls(&self) -> Vec<Call> {
        self.state.borrow().calls.clone()
    }

    /// The calls that changed something: all but the reads.
    #[must_use]
    pub fn writes(&self) -> Vec<Call> {
        let mut calls = self.calls();
        calls.retain(|call| !matches!(call, Call::Labels(_) | Call::Comments(_)));
        calls
    }

    /// The labels on the pull request now.
    #[must_use]
    pub fn current_labels(&self) -> Vec<String> {
        self.state.borrow().labels.clone()
    }

    /// The comments on the pull request now.
    #[must_use]
    pub fn current_comments(&self) -> Vec<Comment> {
        self.state.borrow().comments.clone()
    }
}

impl PullRequests for FakeGitHub {
    fn labels(&self, pr: u64) -> Result<Vec<String>> {
        let mut state = self.state.borrow_mut();
        state.calls.push(Call::Labels(pr));
        Ok(state.labels.clone())
    }

    fn add_label(&self, pr: u64, name: &str) -> Result<()> {
        let mut state = self.state.borrow_mut();
        state.calls.push(Call::AddLabel(pr, name.to_owned()));
        if !state.labels.iter().any(|label| label == name) {
            state.labels.push(name.to_owned());
        }
        Ok(())
    }

    fn remove_label(&self, pr: u64, name: &str) -> Result<()> {
        let mut state = self.state.borrow_mut();
        state.calls.push(Call::RemoveLabel(pr, name.to_owned()));
        // GitHub answers 404 for a label the pull request does not have.
        let Some(index) = state.labels.iter().position(|label| label == name) else {
            return Err(failed("DELETE label", "HTTP 404"));
        };
        state.labels.remove(index);
        Ok(())
    }

    fn ensure_label(&self, spec: &LabelSpec) -> Result<()> {
        self.state
            .borrow_mut()
            .calls
            .push(Call::EnsureLabel(spec.clone()));
        Ok(())
    }

    fn comments(&self, pr: u64) -> Result<Vec<Comment>> {
        let mut state = self.state.borrow_mut();
        state.calls.push(Call::Comments(pr));
        Ok(state.comments.clone())
    }

    fn comment(&self, pr: u64, body: &str) -> Result<()> {
        let mut state = self.state.borrow_mut();
        state.calls.push(Call::Comment(pr, body.to_owned()));
        if state.comments_fail {
            return Err(failed("POST comment", "HTTP 503"));
        }
        // One second apart, so that "newest" is well defined.
        let time = format!("2030-01-01T00:00:{:02}Z", state.comments.len());
        state.comments.push(Comment {
            author: "github-actions[bot]".to_owned(),
            body: body.to_owned(),
            created_at: time.clone(),
            updated_at: time,
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fake_keeps_labels_and_comments_and_the_order_of_calls() -> crate::Result<()> {
        let gh = FakeGitHub::new().with_labels(&["needs-jann"]);
        gh.add_label(7, "approved")?;
        gh.add_label(7, "approved")?;
        assert_eq!(gh.labels(7)?, ["needs-jann", "approved"]);
        gh.remove_label(7, "approved")?;
        assert!(gh.remove_label(7, "approved").is_err());
        gh.comment(7, "one")?;
        gh.comment(7, "two")?;
        let comments = gh.comments(7)?;
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].author, "github-actions[bot]");
        assert_eq!(comments[0].created_at, comments[0].updated_at);
        assert!(comments[0].created_at < comments[1].created_at);
        let spec = LabelSpec {
            name: "x".to_owned(),
            color: "000000".to_owned(),
            description: String::new(),
        };
        gh.ensure_label(&spec)?;
        assert_eq!(
            gh.writes(),
            [
                Call::AddLabel(7, "approved".to_owned()),
                Call::AddLabel(7, "approved".to_owned()),
                Call::RemoveLabel(7, "approved".to_owned()),
                Call::RemoveLabel(7, "approved".to_owned()),
                Call::Comment(7, "one".to_owned()),
                Call::Comment(7, "two".to_owned()),
                Call::EnsureLabel(spec),
            ]
        );
        assert_eq!(
            gh.calls().len(),
            9,
            "seven writes, a read of labels and one of comments"
        );
        assert_eq!(gh.current_labels(), ["needs-jann"]);
        Ok(())
    }

    #[test]
    fn a_failing_fake_refuses_to_write_a_comment() {
        let gh = FakeGitHub::new().with_failing_comments();
        assert!(gh.comment(7, "x").is_err());
        assert_eq!(gh.current_comments(), Vec::<Comment>::new());
        assert_eq!(gh.calls(), [Call::Comment(7, "x".to_owned())]);
    }
}
