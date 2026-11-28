# Word-diff oracle pairs

Synthetic fixtures (P46), not canon. Each `<name>.old.md` and `<name>.new.md` pair is compared
by `theos_content::diff::words` and by `git diff --no-index --word-diff=porcelain
--word-diff-regex='[^[:space:]]+'`; the words both delete and insert must be the same.

Every pair has one alignment only: each changed word differs from the words next to it, so
Myers and git's xdiff cannot choose different but equally short diffs.
