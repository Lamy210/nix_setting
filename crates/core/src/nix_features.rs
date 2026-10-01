pub(crate) fn has_required_flake_features(output: &str) -> bool {
    let mut have_nix_command = false;
    let mut have_flakes = false;

    for token in output.split_whitespace() {
        match token {
            "nix-command" => have_nix_command = true,
            "flakes" => have_flakes = true,
            _ => {}
        }
    }

    have_nix_command && have_flakes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_flake_features_accept_both_tokens_in_any_order() {
        assert!(has_required_flake_features(
            "experimental-features = nix-command flakes"
        ));
        assert!(has_required_flake_features(
            "experimental-features = flakes nix-command"
        ));
    }

    #[test]
    fn required_flake_features_reject_missing_or_lookalike_tokens() {
        assert!(!has_required_flake_features(
            "experimental-features = flakes"
        ));
        assert!(!has_required_flake_features(
            "experimental-features = nix-command"
        ));
        assert!(!has_required_flake_features(
            "experimental-features = nix-command flakes-extra"
        ));
    }
}
