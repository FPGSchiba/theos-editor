//! The one list of options every git read starts with, so a user's git settings cannot change
//! what theos parses.

use std::ffi::OsStr;

/// The options that come before the command of every read: no optional locks, literal pathspecs
/// (a path with `*`, `?`, `[` or `:` is a path, never a pattern), and the settings that would
/// change the output (colour, signatures, the root commit, quoting, encoding).
const OPTIONS: [&str; 12] = [
    "--no-optional-locks",
    "--literal-pathspecs",
    "-c",
    "color.ui=never",
    "-c",
    "log.showSignature=false",
    "-c",
    "log.showRoot=true",
    "-c",
    "core.quotePath=false",
    "-c",
    "i18n.logOutputEncoding=UTF-8",
];

/// The arguments of a read: the read options, then `command`, then `flags`.
pub(crate) fn read_args<'a>(command: &'a str, flags: &[&'a str]) -> Vec<&'a OsStr> {
    OPTIONS
        .iter()
        .copied()
        .chain(std::iter::once(command))
        .chain(flags.iter().copied())
        .map(OsStr::new)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_options_come_first_then_the_command_then_its_flags() {
        let args = read_args("ls-files", &["-z", "--", "a"]);
        let text: Vec<&str> = args.iter().filter_map(|arg| arg.to_str()).collect();
        assert_eq!(
            text,
            [
                "--no-optional-locks",
                "--literal-pathspecs",
                "-c",
                "color.ui=never",
                "-c",
                "log.showSignature=false",
                "-c",
                "log.showRoot=true",
                "-c",
                "core.quotePath=false",
                "-c",
                "i18n.logOutputEncoding=UTF-8",
                "ls-files",
                "-z",
                "--",
                "a",
            ]
        );
    }
}
