//! Credential descriptors, never credentials or arbitrary shell commands.
use clap::Args;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub(crate) struct Options {
    /// Named credential set: built-ins are work, code, and logs
    #[arg(long, default_value = "work")]
    pub profile: String,
    /// Explicit JSON profile file; never reads a provider's credential store
    #[arg(long)]
    pub config: Option<PathBuf>,
    /// Check current access without explicitly refreshing or logging in
    #[arg(long)]
    pub check: bool,
    /// Do not initiate browser/MFA login; silent OAuth refresh remains allowed
    #[arg(long)]
    pub non_interactive: bool,
    /// Refresh known tokens with fewer than this many seconds remaining
    #[arg(long, default_value = "300", value_parser = clap::value_parser!(u64).range(0..=86400))]
    pub min_valid_for: u64,
    /// Deadline in seconds for each status or silent refresh command
    #[arg(long, default_value = "30", value_parser = clap::value_parser!(u64).range(1..=600))]
    pub timeout: u64,
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
        let logs = vec![Target::Slauth {
            audience: "splunk.paas-inf.net".into(),
            environment: "prod".into(),
            groups: vec!["atlassian-all".into()],
            mfa: true,
        }];
        match options.profile.as_str() {
            "code" => code,
            "logs" => logs,
            "work" => code.into_iter().chain(logs).collect(),
            _ => {
                return Err(
                    "Unknown built-in auth profile; use work, code, logs, or --config".into(),
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
