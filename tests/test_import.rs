use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

// The project is currently a binary crate. Include the production modules so
// these integration tests exercise their real parsing and import logic. The
// command boundary is replaced with a byte-oriented fake; no sudo, HOME, or
// machine WireGuard state is used by these tests.
#[path = "../src/config.rs"]
#[allow(dead_code)]
mod config;

mod commands {
    use anyhow::Result;
    use std::fs;
    use std::path::Path;

    pub struct CommandExecutor;

    impl CommandExecutor {
        pub fn install_config(contents: &[u8], target: &Path) -> Result<()> {
            if target.file_name().and_then(|name| name.to_str()) == Some("fail-install.conf") {
                anyhow::bail!("simulated privileged installer failure");
            }
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(target, contents)?;
            Ok(())
        }

        pub fn read_file(path: &Path) -> Result<String> {
            Ok(fs::read_to_string(path)?)
        }

        pub fn ensure_secure_config_path(_path: &Path) -> Result<()> {
            Ok(())
        }

        pub fn open_url(_url: &str) -> Result<()> {
            Ok(())
        }

        pub fn list_config_names(dir: &Path) -> Result<Vec<String>> {
            let mut names = Vec::new();
            for entry in fs::read_dir(dir)? {
                let path = entry?.path();
                if path.extension().and_then(|extension| extension.to_str()) == Some("conf") {
                    if let Some(name) = path.file_stem().and_then(|name| name.to_str()) {
                        names.push(name.to_string());
                    }
                }
            }
            Ok(names)
        }
    }
}

#[path = "../src/download.rs"]
#[allow(dead_code)]
mod download;

use download::{ConfigDownloader, ImportReport, MAX_DISCOVERED_CONFIGS, MAX_WIREGUARD_CONFIG_SIZE};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "wireguard-tui-import-{label}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create test directory");
        Self(path)
    }

    fn join(&self, path: impl AsRef<Path>) -> PathBuf {
        self.0.join(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn valid_config() -> &'static str {
    r#"[Interface]
PrivateKey = AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=
Address = 10.0.0.2/32
DNS = 1.1.1.1

[Peer]
PublicKey = AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=
Endpoint = vpn.example.test:51820
AllowedIPs = 0.0.0.0/0, ::/0
PersistentKeepalive = 25
"#
}

fn assert_send<T: Send>() {}

#[test]
fn import_results_can_cross_a_blocking_task_boundary() {
    assert_send::<ConfigDownloader>();
    assert_send::<ImportReport>();
}

#[test]
fn scans_only_safe_regular_candidates_in_an_explicit_directory() {
    let root = TestDirectory::new("scan");
    let downloads = root.join("downloads");
    fs::create_dir(&downloads).unwrap();

    let accepted = downloads.join("wg-test.conf");
    fs::write(&accepted, valid_config()).unwrap();
    fs::write(downloads.join("notes.txt"), "not a config").unwrap();
    fs::write(
        downloads.join("this-interface-is-too-long.conf"),
        valid_config(),
    )
    .unwrap();
    fs::write(downloads.join("bad\u{1b}.conf"), valid_config()).unwrap();
    fs::create_dir(downloads.join("directory.conf")).unwrap();

    let oversized = downloads.join("oversized.conf");
    let oversized_file = fs::File::create(&oversized).unwrap();
    oversized_file
        .set_len(MAX_WIREGUARD_CONFIG_SIZE + 1)
        .unwrap();

    #[cfg(unix)]
    std::os::unix::fs::symlink(&accepted, downloads.join("symlink.conf")).unwrap();

    let downloader = ConfigDownloader::with_downloads_dir(&downloads);
    let found = downloader.scan_downloads().unwrap();

    assert_eq!(found, vec![accepted]);
}

#[test]
fn scan_reports_an_explicit_candidate_resource_limit() {
    let root = TestDirectory::new("scan-limit");
    let downloads = root.join("downloads");
    fs::create_dir(&downloads).unwrap();
    for index in 0..=MAX_DISCOVERED_CONFIGS {
        fs::write(downloads.join(format!("wg-{index}.conf")), "").unwrap();
    }

    let error = ConfigDownloader::with_downloads_dir(&downloads)
        .scan_downloads()
        .unwrap_err();
    assert!(error.to_string().contains("configuration candidates"));
    assert!(error
        .to_string()
        .contains(&MAX_DISCOVERED_CONFIGS.to_string()));
}

#[test]
fn batch_import_reports_successes_and_security_failures() {
    let root = TestDirectory::new("batch");
    let downloads = root.join("downloads");
    let target = root.join("wireguard");
    fs::create_dir(&downloads).unwrap();
    fs::create_dir(&target).unwrap();

    let valid_path = downloads.join("wg-good.conf");
    let hook_path = downloads.join("wg-hook.conf");
    let installer_failure_path = downloads.join("fail-install.conf");
    fs::write(&valid_path, valid_config()).unwrap();
    fs::write(
        &hook_path,
        "[Interface]\nPrivateKey = AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=\nPostUp = id\n[Peer]\nPublicKey = AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=\n",
    )
    .unwrap();
    fs::write(&installer_failure_path, valid_config()).unwrap();

    let downloader = ConfigDownloader::with_downloads_dir(&downloads);
    let report = downloader
        .import_configs(
            &[
                valid_path.clone(),
                hook_path.clone(),
                installer_failure_path.clone(),
            ],
            &target,
        )
        .unwrap();

    assert_eq!(report.imported, vec!["wg-good.conf"]);
    assert_eq!(report.imported.len(), 1);
    assert_eq!(report.failures.len(), 2);
    assert_eq!(report.failures[0].path, hook_path);
    assert!(report.failures[0].error.contains("hook"));
    assert_eq!(report.failures[1].path, installer_failure_path);
    assert!(report.failures[1]
        .error
        .contains("simulated privileged installer failure"));
    assert_eq!(
        fs::read_to_string(target.join("wg-good.conf")).unwrap(),
        valid_config()
    );
    assert!(!target.join("wg-hook.conf").exists());

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(valid_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn batch_import_reports_duplicate_selections_without_installing_twice() -> Result<()> {
    let root = TestDirectory::new("duplicate-selection");
    let downloads = root.join("downloads");
    let target = root.join("wireguard");
    fs::create_dir(&downloads)?;
    fs::create_dir(&target)?;
    let selected = downloads.join("wg-once.conf");
    let alternate_dir = root.join("alternate");
    fs::create_dir(&alternate_dir)?;
    let duplicate_target = alternate_dir.join("wg-once.conf");
    fs::write(&selected, valid_config())?;
    fs::write(&duplicate_target, valid_config())?;

    let report = ConfigDownloader::with_downloads_dir(&downloads)
        .import_configs(&[selected.clone(), duplicate_target.clone()], &target)?;
    assert_eq!(report.imported, vec!["wg-once.conf"]);
    assert_eq!(report.failures.len(), 1);
    assert_eq!(report.failures[0].path, duplicate_target);
    assert!(report.failures[0]
        .error
        .contains("Duplicate target interface"));
    Ok(())
}

#[test]
fn direct_import_rejects_malformed_and_ambiguous_keys() -> Result<()> {
    let root = TestDirectory::new("invalid-keys");
    let downloads = root.join("downloads");
    let target = root.join("wireguard");
    fs::create_dir(&downloads)?;
    fs::create_dir(&target)?;
    let downloader = ConfigDownloader::with_downloads_dir(&downloads);

    let malformed = downloads.join("malformed.conf");
    fs::write(
        &malformed,
        "[Interface]\nPrivateKey = not-a-key\n[Peer]\nPublicKey = AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=\n",
    )?;
    let error = downloader.import_config(&malformed, &target).unwrap_err();
    assert!(error.to_string().contains("canonical WireGuard key"));

    let duplicate = downloads.join("duplicate.conf");
    fs::write(
        &duplicate,
        "[Interface]\nPrivateKey = AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=\nPrivateKey = AwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwM=\n[Peer]\nPublicKey = AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=\n",
    )?;
    let error = downloader.import_config(&duplicate, &target).unwrap_err();
    assert!(error.to_string().contains("repeats PrivateKey"));

    assert!(!target.join("malformed.conf").exists());
    assert!(!target.join("duplicate.conf").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn direct_import_rejects_a_symlink_before_installation() {
    use std::os::unix::fs::symlink;

    let root = TestDirectory::new("symlink-import");
    let downloads = root.join("downloads");
    let target = root.join("wireguard");
    fs::create_dir(&downloads).unwrap();
    fs::create_dir(&target).unwrap();

    let real = downloads.join("real.conf");
    let link = downloads.join("linked.conf");
    fs::write(&real, valid_config()).unwrap();
    symlink(&real, &link).unwrap();

    let downloader = ConfigDownloader::with_downloads_dir(&downloads);
    let error = downloader.import_config(&link, &target).unwrap_err();

    assert!(error.to_string().contains("not a regular file"));
    assert!(!target.join("linked.conf").exists());
}

#[test]
fn direct_import_enforces_the_size_limit_even_if_scan_is_bypassed() -> Result<()> {
    let root = TestDirectory::new("oversized-import");
    let downloads = root.join("downloads");
    let target = root.join("wireguard");
    fs::create_dir(&downloads)?;
    fs::create_dir(&target)?;

    let source = downloads.join("too-big.conf");
    fs::File::create(&source)?.set_len(MAX_WIREGUARD_CONFIG_SIZE + 1)?;

    let downloader = ConfigDownloader::with_downloads_dir(&downloads);
    let error = downloader.import_config(&source, &target).unwrap_err();
    assert!(error.to_string().contains("byte limit"));
    assert!(!target.join("too-big.conf").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn direct_import_rejects_hard_linked_private_key_files() -> Result<()> {
    let root = TestDirectory::new("hard-link");
    let downloads = root.join("downloads");
    let target = root.join("wireguard");
    fs::create_dir(&downloads)?;
    fs::create_dir(&target)?;

    let original = downloads.join("original.conf");
    let linked = downloads.join("linked.conf");
    fs::write(&original, valid_config())?;
    fs::hard_link(&original, &linked)?;

    let error = ConfigDownloader::with_downloads_dir(&downloads)
        .import_config(&linked, &target)
        .unwrap_err();
    assert!(error.to_string().contains("multiple hard links"));
    assert!(!target.join("linked.conf").exists());
    Ok(())
}
