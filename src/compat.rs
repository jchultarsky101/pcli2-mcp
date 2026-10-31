//! Startup self-check: confirm the pcli2 binary the server will drive is present
//! and that it supports the subcommands this server's tool table depends on.
//!
//! The check is deliberately cheap and *offline*. It runs `pcli2 --version` for
//! the version and `pcli2 <anchor> --help` for a few of the *newest* subcommands
//! per group. Presence is detected from the exit status: an unknown subcommand
//! makes pcli2 print "unrecognized subcommand" and exit non-zero, which is more
//! robust than parsing human-readable help prose and survives new pcli2
//! versions. The heavier, networked `pcli2 doctor` check stays available as an
//! on-demand MCP tool; it is not put on the startup path.
//!
//! A missing binary is the only case that stops the start hard, because a server
//! that wraps a CLI which isn't there cannot function. A below-minimum version or
//! a missing anchor subcommand is reported to the operator (the log) and to the
//! model (the `initialize` `instructions`), but does not by itself refuse the
//! start.

use std::time::Duration;

use tokio::process::Command;
use tokio::time::timeout;

/// Minimum pcli2 version the tool table requires. The command groups added in
/// 0.3.0 (catalog, report, tenant-failures, folder-description, …) all land at or
/// after pcli2 2.4–2.8, and the CHANGELOG pins "requires pcli2 2.8 or newer".
pub const MIN_SUPPORTED: (u32, u32, u32) = (2, 8, 0);

/// The pcli2 versions this build was verified against (informational only).
pub const TESTED_AGAINST: &str = "2.8.1 - 2.8.2";

/// An optional environment variable that disables the startup check entirely.
pub const DISABLE_ENV: &str = "PCLI2_MCP_NO_STARTUP_CHECK";

// Per-call wall-clock ceiling. `--version` / `--help` are local and instantaneous,
// so a hang here means pcli2 is broken, not slow.
const CALL_TIMEOUT: Duration = Duration::from_secs(15);

/// A pcli2 subcommand whose presence we spot-check at startup. `argv` is the
/// subcommand path *after* the pcli2 executable.
#[derive(Clone, Copy, Debug)]
pub struct Anchor {
    pub label: &'static str,
    pub argv: &'static [&'static str],
}

/// Anchors, newest-group-first, covering the tool groups that the 0.3.0 release
/// added on top of 0.2.0. All were verified present on pcli2 2.8.2. These exist
/// so the check can catch a binary that is *too old* (a group the table needs is
/// absent) or *changed by a newer breaking release* (a group we rely on got
/// removed) — the two cases a bare version number cannot see.
pub const ANCHORS: [Anchor; 6] = [
    Anchor {
        label: "catalog list",
        argv: &["catalog", "list"],
    },
    Anchor {
        label: "report list",
        argv: &["report", "list"],
    },
    Anchor {
        label: "tenant failures",
        argv: &["tenant", "failures"],
    },
    Anchor {
        label: "folder description get",
        argv: &["folder", "description", "get"],
    },
    Anchor {
        label: "asset move",
        argv: &["asset", "move"],
    },
    Anchor {
        label: "user list",
        argv: &["user", "list"],
    },
];

/// The outcome of a startup self-check against one pcli2 binary.
///
/// The derived default is the "not run" report: empty, and it renders no
/// `instructions`, so a state built without a check (or with the check
/// disabled) adds no noise to `initialize`.
#[derive(Clone, Default, Debug)]
pub struct Report {
    /// The pcli2 executable that was resolved (`PCLI2_BIN`, or `pcli2`).
    pub binary: String,
    /// Whether `pcli2 --version` could be executed (the binary was found and ran).
    pub present: bool,
    /// The parsed semantic version `(major, minor, patch)`, when parseable.
    pub version: Option<(u32, u32, u32)>,
    /// `(anchor label, present?)` for each anchor that was checked.
    pub anchors: Vec<(String, bool)>,
}

impl Report {
    /// Whether the detected version meets the minimum. A missing or unparseable
    /// version counts as *not* satisfied.
    pub fn supports_min(&self) -> bool {
        self.version.is_some_and(|v| v >= MIN_SUPPORTED)
    }

    /// Whether the check passed overall: pcli2 present, version at/above the
    /// minimum, and every anchor subcommand present.
    pub fn ok(&self) -> bool {
        self.present && self.supports_min() && self.missing_anchors().is_empty()
    }

    /// Anchor labels that are missing on the running pcli2.
    pub fn missing_anchors(&self) -> Vec<String> {
        self.anchors
            .iter()
            .filter(|(_, present)| !*present)
            .map(|(label, _)| label.clone())
            .collect()
    }

    /// How many anchors were found present.
    pub fn present_anchors(&self) -> usize {
        self.anchors.iter().filter(|(_, present)| *present).count()
    }

    /// A stable "A.B.C" rendering of the version, or `"unknown"`.
    pub fn version_string(&self) -> String {
        match self.version {
            Some((ma, mi, pa)) => format!("{ma}.{mi}.{pa}"),
            None => "unknown".to_string(),
        }
    }

    /// A concise, English description of the check for the model, suitable as the
    /// `initialize` `instructions`. Returns empty when the check did not run, so
    /// a bare or check-disabled `initialize` stays unchanged.
    pub fn instructions(&self) -> String {
        if self.anchors.is_empty() {
            return String::new();
        }

        let mut out = String::from("This MCP server wraps the pcli2 CLI v");
        out.push_str(&self.version_string());

        let verdict = match self.version {
            Some(v) if v >= MIN_SUPPORTED => format!(
                ". Supported: pcli2 meets the required 2.8.0 (tested {TESTED_AGAINST}); "
            ),
            Some(_) =>
                "Below the required pcli2 2.8.0; some tools added in 0.3.0 will fail until pcli2 is updated. ".to_string(),
            None => "pcli2's version could not be determined and it may be incompatible. ".to_string(),
        };
        out.push_str(&verdict);

        let total = ANCHORS.len();
        let missing = self.missing_anchors();
        if missing.is_empty() {
            out.push_str(&format!(
                "All {total} expected subcommand groups are present. "
            ));
        } else {
            out.push_str(&format!(
                "{} of {total} expected subcommand groups are missing ({}) and their tools will fail. ",
                missing.len(),
                missing.join(", ")
            ));
        }

        out.push_str("pcli2 runs non-interactively as the locally configured user.");
        out
    }
}

/// Parse a semantic version out of free-form text such as `pcli2 2.8.2` or
/// `v2.8.0-rc1`. Components after the first two missing ones default to `0`; a
/// pre-release/build suffix is ignored. Returns `None` when no version-like token
/// is found.
pub fn parse_version(input: &str) -> Option<(u32, u32, u32)> {
    for token in input.split_whitespace() {
        let rest = token.strip_prefix('v').unwrap_or(token);
        // Take the leading run of digits and dots; stop at a pre-release/build
        // suffix or any non-numeric character.
        let core: String = rest
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        if core.is_empty() {
            continue;
        }
        let parts: Vec<&str> = core.split('.').collect();
        let major = parts[0].parse().ok()?;
        let minor = parts
            .get(1)
            .and_then(|p| p.parse::<u32>().ok())
            .unwrap_or(0);
        let patch = parts
            .get(2)
            .and_then(|p| p.parse::<u32>().ok())
            .unwrap_or(0);
        return Some((major, minor, patch));
    }
    None
}

/// Whether the startup check should be skipped (`PCLI2_MCP_NO_STARTUP_CHECK` is
/// set to a truthy value).
pub fn disabled() -> bool {
    let Ok(raw) = std::env::var(DISABLE_ENV) else {
        return false;
    };
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "on" | "yes" | "y"
    )
}

/// Run the startup self-check against `binary`, which is the resolved pcli2
/// executable (from `PCLI2_BIN` or `PATH`).
///
/// Returns a [`Report`]. `present` is false when the binary cannot be executed
/// (the caller should treat that as a hard start failure); a present binary with a
/// below-minimum version or missing anchors still yields a populated report.
pub async fn check(binary: &str) -> Report {
    let mut present = false;
    let mut version = None;
    let mut anchors: Vec<(String, bool)> = Vec::with_capacity(ANCHORS.len());

    if let Some(out) = run(binary, &["--version"]).await {
        present = true;
        version = parse_version(&out.stdout);
    }

    // Append `--help` so the presence probe is argument- and auth-independent.
    // `pcli2 <group> ... --help` prints usage and exits 0 when the subcommand
    // exists, and exits non-zero ("unrecognized subcommand") when it does not,
    // regardless of whether the subcommand would otherwise need a name, a
    // tenant, or connectivity. That exit status is the offline signal the check
    // depends on; running the bare subcommand would instead fail on a missing
    // argument, which is not a reliable "is the group present?" answer.
    for anchor in &ANCHORS {
        let mut argv = anchor.argv.to_vec();
        argv.push("--help");
        let present = run(binary, &argv).await.is_some_and(|out| out.ok);
        anchors.push((anchor.label.to_string(), present));
    }

    Report {
        binary: binary.to_string(),
        present,
        version,
        anchors,
    }
}

struct RunOut {
    ok: bool,
    stdout: String,
}

/// Executes `<binary> <args...>` with the same non-interactive environment the
/// tool runs use, bounded by [`CALL_TIMEOUT`]. Returns `None` when the binary
/// could not be spawned (not found / not executable) or timed out.
async fn run(binary: &str, args: &[&str]) -> Option<RunOut> {
    let mut command = Command::new(binary);
    for arg in args {
        command.arg(arg);
    }
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .env("PCLI2_NO_INPUT", "1")
        .env("PCLI2_NO_COLOR", "1")
        .env("PCLI2_NO_UPDATE_CHECK", "1");

    let child = match command.spawn() {
        Ok(child) => child,
        Err(_) => return None,
    };

    match timeout(CALL_TIMEOUT, child.wait_with_output()).await {
        Ok(Ok(output)) => Some(RunOut {
            ok: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        }),
        // The binary spawned but could not be reaped; treat as a failed check.
        Ok(Err(_)) => Some(RunOut {
            ok: false,
            stdout: String::new(),
        }),
        // Timed out: no verdict, but the binary at least existed.
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version_basic() {
        assert_eq!(parse_version("pcli2 2.8.2"), Some((2, 8, 2)));
        assert_eq!(parse_version("pcli2 2.8"), Some((2, 8, 0)));
        assert_eq!(parse_version("pcli2 v2.8.0-rc1"), Some((2, 8, 0)));
        assert_eq!(parse_version("pcli2 1.0.0"), Some((1, 0, 0)));
        assert_eq!(parse_version("just some text"), None);
        assert_eq!(parse_version(""), None);
    }

    #[test]
    fn anchors_are_a_nonempty_distinct_set() {
        assert_eq!(ANCHORS.len(), 6);
        let labels: Vec<&str> = ANCHORS.iter().map(|a| a.label).collect();
        labels.iter().for_each(|l| assert!(!l.is_empty()));
        let mut distinct = labels.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct.len(), labels.len(), "duplicate anchor labels");
        // Every anchor argv begins with a known top-level group.
        let groups = [
            "catalog",
            "report",
            "tenant",
            "folder",
            "asset",
            "user",
            "auth",
            "environment",
            "config",
            "cache",
            "api",
        ];
        for anchor in &ANCHORS {
            assert!(
                anchor
                    .argv
                    .first()
                    .is_some_and(|first| groups.contains(first)),
                "anchor {:?} has an unexpected leading subcommand",
                anchor.label
            );
        }
    }

    #[test]
    fn empty_report_is_quiet_and_not_ok() {
        let r = Report::default();
        assert!(r.instructions().is_empty());
        assert!(!r.ok());
        assert!(!r.supports_min());
        assert!(r.missing_anchors().is_empty());
        assert_eq!(r.version_string(), "unknown");
    }

    #[test]
    fn supported_report_reads_well() {
        let r = Report {
            binary: "pcli2".to_string(),
            present: true,
            version: Some((2, 8, 2)),
            anchors: ANCHORS
                .iter()
                .map(|a| (a.label.to_string(), true))
                .collect(),
        };
        assert!(r.ok());
        assert!(r.supports_min());
        let ins = r.instructions();
        assert!(ins.contains("2.8.2"), "{ins}");
        assert!(ins.contains("Supported"), "{ins}");
        assert!(
            ins.contains("All 6 expected subcommand groups are present"),
            "{ins}"
        );
        assert!(ins.contains("2.8.0"), "{ins}");
    }

    #[test]
    fn below_minimum_report_is_flagged() {
        let r = Report {
            binary: "pcli2".to_string(),
            present: true,
            version: Some((2, 7, 9)),
            anchors: ANCHORS
                .iter()
                .map(|a| (a.label.to_string(), true))
                .collect(),
        };
        assert!(!r.ok());
        assert!(!r.supports_min());
        let ins = r.instructions();
        assert!(ins.contains("2.7.9"), "{ins}");
        assert!(
            ins.to_ascii_lowercase().contains("below the required"),
            "{ins}"
        );
    }

    #[test]
    fn missing_anchors_are_reported() {
        // 2 present, 1 missing: the "N of total" count must be the *missing*
        // count (1), not the present count (2). This guards the exact bug that
        // an earlier draft had, where the present count was printed.
        let r = Report {
            binary: "pcli2".to_string(),
            present: true,
            version: Some((2, 8, 2)),
            anchors: vec![
                ("catalog list".to_string(), true),
                ("report list".to_string(), false),
                ("user list".to_string(), true),
            ],
        };
        assert_eq!(r.missing_anchors(), vec!["report list".to_string()]);
        assert_eq!(r.present_anchors(), 2);
        let ins = r.instructions();
        assert!(ins.contains("report list"), "{ins}");
        assert!(
            ins.contains("1 of 6"),
            "missing count must be printed, got: {ins}"
        );
        assert!(
            !ins.contains("2 of 6"),
            "must not print the present count, got: {ins}"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn check_exercises_a_fake_pcli2() {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!("pcli2-mcp-compat-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let fake = dir.join("fake-pcli2");
        let script =
            "#!/bin/sh\ncase \"$1\" in\n  --version) echo 'pcli2 2.8.2' ;;\nesac\nexit 0\n";
        let mut f = std::fs::File::create(&fake).unwrap();
        f.write_all(script.as_bytes()).unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();

        let report = check(fake.to_str().unwrap()).await;
        drop(std::fs::remove_dir_all(&dir));

        assert!(report.present, "the spawned fake binary should be present");
        assert_eq!(
            report.version,
            Some((2, 8, 2)),
            "version parsed from stdout"
        );
        assert!(
            report.missing_anchors().is_empty(),
            "every anchor exits 0 for the always-succeeding fake; found {:?}",
            report.missing_anchors()
        );
        assert!(report.ok());
        assert!(report.instructions().contains("2.8.2"));
    }
}
