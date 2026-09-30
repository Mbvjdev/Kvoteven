pub fn valid_proof(enabled: bool, demo: bool, providers: &[String], language: &str) -> bool {
    enabled
        && demo
        && language == "en,da"
        && providers.len() == 2
        && providers.iter().any(|p| p == "deepseek")
        && providers.iter().any(|p| p == "openai-codex")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_smoke_proves_both_providers_and_languages() {
        let providers = vec!["deepseek".into(), "openai-codex".into()];
        assert!(valid_proof(true, true, &providers, "en,da"));
        assert!(!valid_proof(false, true, &providers, "en,da"));
        assert!(!valid_proof(true, false, &providers, "en,da"));
        assert!(!valid_proof(true, true, &providers, "en"));
        assert!(!valid_proof(
            true,
            true,
            &["deepseek".into(), "deepseek".into()],
            "en,da"
        ));
        assert!(!valid_proof(true, true, &[], "en,da"));
    }
}
