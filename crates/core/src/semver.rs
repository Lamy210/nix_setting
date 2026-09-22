//! Strict SemVer 2.0.0 parsing and precedence helpers used by release-facing core code.
//!
//! This module intentionally stays dependency-free. Numeric identifiers are compared by
//! decimal string length and lexical order so valid SemVer values are not bounded by a
//! machine integer width.

use std::cmp::Ordering;

pub(crate) fn is_prerelease(version: &str) -> Option<bool> {
    let (_, prerelease) = parse(version)?;
    Some(prerelease.is_some())
}

pub(crate) fn compare(a: &str, b: &str) -> Option<Ordering> {
    let (a_core, a_pre) = parse(a)?;
    let (b_core, b_pre) = parse(b)?;

    Some(
        compare_core(a_core, b_core).then_with(|| match (a_pre, b_pre) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(a_pre), Some(b_pre)) => compare_prerelease(a_pre, b_pre),
        }),
    )
}

fn parse(version: &str) -> Option<([&str; 3], Option<&str>)> {
    let without_build = match version.split_once('+') {
        Some((base, build)) => {
            if build.contains('+') || !valid_dot_identifiers(build, false) {
                return None;
            }
            base
        }
        None => version,
    };

    let (core, prerelease) = match without_build.split_once('-') {
        Some((core, prerelease)) => {
            if !valid_dot_identifiers(prerelease, true) {
                return None;
            }
            (core, Some(prerelease))
        }
        None => (without_build, None),
    };

    let mut parts = core.split('.');
    let major = validate_core_identifier(parts.next()?)?;
    let minor = validate_core_identifier(parts.next()?)?;
    let patch = validate_core_identifier(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }

    Some(([major, minor, patch], prerelease))
}

fn validate_core_identifier(value: &str) -> Option<&str> {
    if value.is_empty()
        || !value.bytes().all(|b| b.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return None;
    }
    Some(value)
}

fn valid_dot_identifiers(value: &str, reject_numeric_leading_zero: bool) -> bool {
    !value.is_empty()
        && value.split('.').all(|identifier| {
            !identifier.is_empty()
                && identifier
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
                && !(reject_numeric_leading_zero
                    && identifier.len() > 1
                    && identifier.starts_with('0')
                    && identifier.bytes().all(|b| b.is_ascii_digit()))
        })
}

fn compare_numeric_identifier(a: &str, b: &str) -> Ordering {
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}

fn compare_core(a: [&str; 3], b: [&str; 3]) -> Ordering {
    for (a_part, b_part) in a.into_iter().zip(b) {
        let ordering = compare_numeric_identifier(a_part, b_part);
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    Ordering::Equal
}

fn compare_prerelease(a: &str, b: &str) -> Ordering {
    let mut a_parts = a.split('.');
    let mut b_parts = b.split('.');

    loop {
        match (a_parts.next(), b_parts.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(a_part), Some(b_part)) => {
                let a_numeric = a_part.bytes().all(|b| b.is_ascii_digit());
                let b_numeric = b_part.bytes().all(|b| b.is_ascii_digit());
                let ordering = match (a_numeric, b_numeric) {
                    (true, true) => compare_numeric_identifier(a_part, b_part),
                    (true, false) => Ordering::Less,
                    (false, true) => Ordering::Greater,
                    (false, false) => a_part.cmp(b_part),
                };
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_stable_preview_and_build_metadata() {
        assert_eq!(is_prerelease("1.2.3"), Some(false));
        assert_eq!(is_prerelease("1.2.3+build-5"), Some(false));
        assert_eq!(is_prerelease("1.2.3-rc.1"), Some(true));
        assert_eq!(is_prerelease("1.2.3--foo"), Some(true));
    }

    #[test]
    fn rejects_invalid_semver_forms() {
        for invalid in [
            "01.2.3",
            "1.02.3",
            "1.2.03",
            "1.2.3-01",
            "1.2.3-alpha..1",
            "1٢.2.3",
            "1.2.3-",
            "1.2",
            "1.2.3+",
        ] {
            assert_eq!(is_prerelease(invalid), None, "{invalid} must be rejected");
        }
    }

    #[test]
    fn compares_semver_precedence_without_integer_overflow() {
        assert_eq!(compare("1.0.0-1", "1.0.0-alpha"), Some(Ordering::Less));
        assert_eq!(
            compare("1.0.0-alpha.9", "1.0.0-alpha.10"),
            Some(Ordering::Less)
        );
        assert_eq!(
            compare("1.0.0-rc.10+build.1", "1.0.0-rc.10+build.2"),
            Some(Ordering::Equal),
            "build metadata must not affect precedence"
        );
        assert_eq!(
            compare("184467440737095516160.0.0", "184467440737095516159.999.999",),
            Some(Ordering::Greater)
        );
    }

    #[test]
    fn follows_semver_prerelease_precedence_chain() {
        let ordered = [
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-alpha.beta",
            "1.0.0-beta",
            "1.0.0-beta.2",
            "1.0.0-beta.11",
            "1.0.0-rc.1",
            "1.0.0",
        ];

        for pair in ordered.windows(2) {
            assert_eq!(compare(pair[0], pair[1]), Some(Ordering::Less), "{pair:?}");
        }
    }
}
