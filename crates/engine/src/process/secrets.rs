//! Masking credentials out of a supervised process's argv.
//!
//! The game takes its token no way but the command line, so the exposure is
//! already there for anything reading the process table. What this prevents is
//! that exposure outliving the session or travelling past it.

/// The argv flags whose *following* token is a credential.
const SECRET_FLAGS: &[&str] = &["--accessToken", "--session"];

const MASK: &str = "<redacted>";

/// `args` with every credential replaced by a fixed mask.
pub fn redact(args: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(args.len());
    let mut mask_next = false;
    for arg in args {
        if std::mem::replace(&mut mask_next, false) {
            out.push(MASK.to_string());
            continue;
        }
        mask_next = SECRET_FLAGS.contains(&arg.as_str());
        out.push(arg.clone());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(raw: &[&str]) -> Vec<String> {
        raw.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn every_credential_flag_masks_its_value() {
        let redacted = redact(&args(&[
            "--username",
            "toraaoo",
            "--accessToken",
            "ey.a.real.jwt",
            "--session",
            "ey.another.one",
            "--versionType",
            "release",
        ]));
        assert_eq!(
            redacted,
            args(&[
                "--username",
                "toraaoo",
                "--accessToken",
                "<redacted>",
                "--session",
                "<redacted>",
                "--versionType",
                "release",
            ])
        );
    }

    #[test]
    fn a_trailing_flag_masks_nothing_and_drops_nothing() {
        assert_eq!(
            redact(&args(&["-cp", "a.jar", "--accessToken"])),
            args(&["-cp", "a.jar", "--accessToken"])
        );
    }

    #[test]
    fn a_value_that_looks_like_a_flag_is_still_masked_once() {
        assert_eq!(
            redact(&args(&["--accessToken", "--session", "real"])),
            args(&["--accessToken", "<redacted>", "real"]),
            "the token after a secret flag is consumed, whatever it looks like"
        );
    }
}
