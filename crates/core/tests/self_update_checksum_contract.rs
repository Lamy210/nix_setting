use schneeforge_core::{expected_sha256, Error};

const ASSET: &str = "schneeforge-x86_64-linux";
const SHA_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SHA_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[test]
fn duplicate_checksum_entries_are_rejected_even_when_hashes_match() {
    let checksums = format!(
        "{SHA_A}  dist/linux/{ASSET}\n{SHA_A}  mirror/{ASSET}\n"
    );

    let err = expected_sha256(&checksums, ASSET).unwrap_err();
    assert!(matches!(err, Error::SelfUpdate(_)), "{err}");
    assert!(err.to_string().contains("複数"), "{err}");
}

#[test]
fn conflicting_checksum_entries_are_rejected() {
    let checksums = format!(
        "{SHA_A}  dist/linux/{ASSET}\n{SHA_B}  mirror/{ASSET}\n"
    );

    let err = expected_sha256(&checksums, ASSET).unwrap_err();
    assert!(matches!(err, Error::SelfUpdate(_)), "{err}");
    assert!(err.to_string().contains("複数"), "{err}");
}
