//! Credential descriptors, never credentials or arbitrary shell commands.
use clap::Args;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub(crate) struct Options {
    /// Named credential set: work, code, logs, services, risk, checks, config, metrics
    #[arg(long, default_value = "work")]
    pub profile: String,
    /// Explicit JSON profile file; never reads a provider's credential store
    #[arg(long)]
    pub config: Option<PathBuf>,
    /// Check current access without explicitly refreshing or logging in
    #[arg(long)]
    pub check: bool,
    /// Reuse valid credentials; by default every target is explicitly renewed
    #[arg(long)]
    pub reuse_valid: bool,
    /// Do not initiate browser/MFA login; silent OAuth refresh remains allowed
    #[arg(long)]
    pub non_interactive: bool,
    /// Refresh known tokens with fewer than this many seconds remaining
    #[arg(long, default_value = "300", value_parser = clap::value_parser!(u64).range(0..=86400))]
    pub min_valid_for: u64,
    /// Deadline in seconds for each status or silent refresh command
    #[arg(long, default_value = "30", value_parser = clap::value_parser!(u64).range(1..=600))]
    pub timeout: u64,
    /// Wait for another preflight in seconds; defaults to --timeout
    #[arg(long, value_parser = clap::value_parser!(u64).range(1..=3600))]
    pub coordination_timeout: Option<u64>,
    /// Deadline in seconds for each browser/MFA login command
    #[arg(long, default_value = "180", value_parser = clap::value_parser!(u64).range(1..=900))]
    pub login_timeout: u64,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Target {
    Bitbucket {
        #[serde(default)]
        account: Option<String>,
    },
    Twg {},
    Slauth {
        audience: String,
        #[serde(default = "production")]
        environment: String,
        #[serde(default)]
        groups: Vec<String>,
        #[serde(default)]
        mfa: bool,
    },
}
fn production() -> String {
    "prod".into()
}
impl Target {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Bitbucket { .. } => "Bitbucket",
            Self::Twg {} => "TWG",
            Self::Slauth { .. } => "SLAuth",
        }
    }
    pub fn scope(&self) -> Option<String> {
        match self {
            Self::Slauth {
                audience,
                environment,
                groups,
                mfa,
            } => {
                let mut parts = vec![environment.clone(), audience.clone()];
                if !groups.is_empty() {
                    parts.push(format!("groups: {}", groups.join(", ")));
                }
                if *mfa {
                    parts.push("MFA".into());
                }
                Some(parts.join(" · "))
            }
            _ => None,
        }
    }
    pub fn label(&self) -> String {
        match self {
            Self::Bitbucket { account } => {
                format!("bitbucket:{}", account.as_deref().unwrap_or("active"))
            }
            Self::Twg {} => "twg".into(),
            Self::Slauth {
                audience,
                environment,
                groups,
                mfa,
            } => format!(
                "slauth:{environment}:{audience}:{}:mfa={mfa}",
                groups.join(",")
            ),
        }
    }
    pub fn validate(&mut self) -> Result<(), String> {
        fn identifier(s: &str) -> bool {
            !s.is_empty()
                && s.len() <= 256
                && s.bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"._:/@-".contains(&c))
                && !s.starts_with('-')
        }
        match self {
            Self::Bitbucket {
                account: Some(account),
            } if !identifier(account) => {
                Err("Bitbucket account must be a nonempty identifier".into())
            }
            Self::Slauth {
                audience,
                environment,
                groups,
                ..
            } => {
                if !identifier(audience)
                    || !["prod", "staging", "dev", "platdev"].contains(&environment.as_str())
                    || groups.len() > 16
                    || !groups.iter().all(|s| identifier(s))
                {
                    return Err("Invalid SLAuth audience, environment, or groups".into());
                }
                groups.sort();
                groups.dedup();
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema_version: u32,
    profiles: BTreeMap<String, Vec<Target>>,
}
pub(crate) fn targets(options: &Options) -> Result<Vec<Target>, String> {
    let mut selected = if let Some(path) = &options.config {
        use std::io::Read;
        use std::os::unix::fs::OpenOptionsExt;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(path)
            .map_err(|_| "Cannot open auth profile file")?;
        if !file
            .metadata()
            .map_err(|_| "Cannot inspect auth profile file")?
            .is_file()
        {
            return Err("Auth profile must be a regular file".into());
        }
        let mut bytes = Vec::new();
        file.take(65537)
            .read_to_end(&mut bytes)
            .map_err(|_| "Cannot read auth profile file")?;
        if bytes.len() > 65536 {
            return Err("Auth profile exceeds 64 KiB".into());
        }
        // Parse errors can contain input strings. Never render those strings.
        let mut config: Config = serde_json::from_slice(&bytes)
            .map_err(|_| "Invalid auth profile JSON or unsupported fields")?;
        if config.schema_version != 1 {
            return Err("Unsupported auth profile schema version".into());
        }
        config
            .profiles
            .remove(&options.profile)
            .ok_or("Auth profile not found")?
    } else {
        let code = vec![Target::Bitbucket { account: None }, Target::Twg {}];
        let slauth = |audience: &str, mfa, groups: &[&str]| Target::Slauth {
            audience: audience.into(),
            environment: "prod".into(),
            groups: groups.iter().map(|s| (*s).into()).collect(),
            mfa,
        };
        let logs = vec![slauth("splunk.paas-inf.net", true, &["atlassian-all"])];
        let risk = slauth("risk-mitigation-service", true, &[]);
        let checks = slauth("pollinator", true, &[]);
        let config = slauth("mlp-config", false, &[]);
        let metrics = slauth("phobos", false, &[]);
        let services = vec![
            risk.clone(),
            checks.clone(),
            config.clone(),
            metrics.clone(),
        ];
        match options.profile.as_str() {
            "code" => code,
            "logs" => logs,
            "risk" => vec![risk],
            "checks" => vec![checks],
            "config" => vec![config],
            "metrics" => vec![metrics],
            "services" => services,
            "work" => code.into_iter().chain(logs).chain(services).collect(),
            _ => {
                return Err(
                    "Unknown built-in auth profile; use work, code, logs, services, risk, checks, config, metrics, or --config".into(),
                )
            }
        }
    };
    if selected.is_empty() || selected.len() > 32 {
        return Err("Auth profile requires 1 to 32 targets".into());
    }
    for target in &mut selected {
        target.validate()?;
    }
    let mut unique = std::collections::BTreeSet::new();
    if !selected.iter().all(|t| unique.insert(t.label())) {
        return Err("Duplicate credential target in auth profile".into());
    }
    Ok(selected)
}
