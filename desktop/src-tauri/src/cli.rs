#[derive(Debug, PartialEq)]
pub enum Mode {
    Help,
    Version,
    App { demo: bool, smoke: bool },
    Check { provider: String, source: String },
}

pub fn parse(args: &[String]) -> Result<Mode, &'static str> {
    const BAD: &str = "Unsupported arguments. Use --help.";
    if args == ["--help"] {
        return Ok(Mode::Help);
    }
    if args == ["--version"] {
        return Ok(Mode::Version);
    }
    let (mut demo, mut smoke, mut check) = (false, false, false);
    let (mut provider, mut source) = (None, None);
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--demo" if !demo => demo = true,
            "--smoke-test" if !smoke => smoke = true,
            "--check" if !check => check = true,
            "--provider" if provider.is_none() => {
                let value = args.next().ok_or(BAD)?;
                if !["deepseek", "openai-codex"].contains(&value.as_str()) {
                    return Err(BAD);
                }
                provider = Some(value.clone());
            }
            "--source" if source.is_none() => {
                let value = args.next().ok_or(BAD)?;
                if !["auto", "codex", "hermes"].contains(&value.as_str()) {
                    return Err(BAD);
                }
                source = Some(value.clone());
            }
            _ => return Err(BAD),
        }
    }
    if check {
        if demo || smoke {
            return Err("Demo cannot be used for a live check.");
        }
        Ok(Mode::Check {
            provider: provider.unwrap_or_else(|| "deepseek".into()),
            source: source.unwrap_or_else(|| "auto".into()),
        })
    } else {
        if provider.is_some() || source.is_some() || (smoke && !demo) {
            return Err(BAD);
        }
        Ok(Mode::App { demo, smoke })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }
    #[test]
    fn normal_start_is_not_demo() {
        assert_eq!(
            parse(&[]),
            Ok(Mode::App {
                demo: false,
                smoke: false
            })
        );
    }
    #[test]
    fn smoke_is_explicitly_offline() {
        assert_eq!(
            parse(&args(&["--demo", "--smoke-test"])),
            Ok(Mode::App {
                demo: true,
                smoke: true
            })
        );
        assert!(parse(&args(&["--smoke-test"])).is_err());
    }
    #[test]
    fn synthetic_data_cannot_pass_live_check() {
        assert!(parse(&args(&["--check", "--demo"])).is_err());
    }
    #[test]
    fn check_arguments_are_allowlisted() {
        assert_eq!(
            parse(&args(&[
                "--check",
                "--provider",
                "openai-codex",
                "--source",
                "hermes"
            ])),
            Ok(Mode::Check {
                provider: "openai-codex".into(),
                source: "hermes".into()
            })
        );
        for a in [
            vec!["--check", "--provider"],
            vec!["--provider", "deepseek"],
            vec!["--check", "--source", "evil"],
            vec!["--check", "--provider", "unknown"],
            vec!["--key", "dummy"],
        ] {
            assert!(parse(&args(&a)).is_err());
        }
    }
    #[test]
    fn help_and_version_are_unambiguous() {
        assert_eq!(parse(&args(&["--help"])), Ok(Mode::Help));
        assert_eq!(parse(&args(&["--version"])), Ok(Mode::Version));
        assert!(parse(&args(&["--help", "--check"])).is_err());
    }
}
