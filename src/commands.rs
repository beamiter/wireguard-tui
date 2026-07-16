use anyhow::{anyhow, Context, Result};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};
#[cfg(unix)]
use std::os::unix::process::CommandExt;

const TRUSTED_EXECUTABLE_DIRS: [&str; 6] = [
    "/usr/bin",
    "/usr/sbin",
    "/bin",
    "/sbin",
    "/usr/local/bin",
    "/usr/local/sbin",
];
const SECRET_REDACTION_EXPRESSION: &str =
    r"s/^([[:space:]]*(PrivateKey|PresharedKey)[[:space:]]*=[[:space:]]*).*/\1<redacted>/I";
const SECRET_VALIDATION_AWK: &str = r#"
BEGIN { bad = 0 }
{
    separator = index($0, "=")
    if (separator == 0) next
    key = substr($0, 1, separator - 1)
    gsub(/^[ \t]+|[ \t]+$/, "", key)
    key = tolower(key)
    if (key != "privatekey" && key != "presharedkey") next
    value = substr($0, separator + 1)
    gsub(/^[ \t]+|[ \t\r]+$/, "", value)
    prefix = substr(value, 1, 42)
    tail = substr(value, 43, 1)
    if (length(value) != 44 || prefix ~ /[^A-Za-z0-9+\/]/ || tail !~ /^[AEIMQUYcgkosw048]$/ || substr(value, 44, 1) != "=" || value == "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=") bad = 1
}
END { exit bad ? 42 : 0 }
"#;
const PRIVILEGED_PATH_DIRS: [&str; 4] = ["/usr/sbin", "/usr/bin", "/sbin", "/bin"];
const MAX_COMMAND_OUTPUT_BYTES: usize = 1024 * 1024;

pub struct CommandExecutor;

impl CommandExecutor {
    /// Validate sudo credentials while the application still owns a normal terminal.
    /// All later privileged commands are deliberately non-interactive.
    pub fn authorize_privileges() -> Result<()> {
        if Self::running_as_root() {
            return Ok(());
        }

        let sudo = Self::resolve_trusted_executable("sudo")?;
        let mut command = Command::new(&sudo);
        Self::configure_environment(&mut command)?;
        let status = command
            .arg("-v")
            .status()
            .with_context(|| format!("failed to start `{} -v`", sudo.display()))?;

        if status.success() {
            Ok(())
        } else {
            Err(anyhow!(
                "privilege authorization failed: `sudo -v` exited with {status}"
            ))
        }
    }

    /// Whether the process itself is running with effective UID 0.
    ///
    /// This intentionally uses the kernel-reported effective UID instead of executing a
    /// PATH-resolved helper such as `id`.
    pub fn running_as_root() -> bool {
        // SAFETY: `geteuid` takes no arguments and has no preconditions.
        unsafe { libc::geteuid() == 0 }
    }

    pub fn check_wireguard_installed() -> Result<bool> {
        Ok(Self::command_exists("wg") && Self::command_exists("wg-quick"))
    }

    pub fn open_url(url: &str) -> Result<()> {
        let executable = Self::resolve_trusted_executable("xdg-open")?;
        let mut child = Command::new(executable)
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("failed to start the system URL opener")?;
        thread::spawn(move || {
            let _ = child.wait();
        });
        Ok(())
    }

    pub fn connect_vpn(config_name: &str) -> Result<String> {
        let mut command = Self::privileged_command("wg-quick")?;
        command.arg("up").arg(config_name);
        Self::run_command_with_timeout(
            command,
            &format!("connect WireGuard interface `{config_name}`"),
            Duration::from_secs(45),
        )
    }

    pub fn disconnect_vpn(config_name: &str) -> Result<String> {
        let mut command = Self::privileged_command("wg-quick")?;
        command.arg("down").arg(config_name);
        Self::run_command_with_timeout(
            command,
            &format!("disconnect WireGuard interface `{config_name}`"),
            Duration::from_secs(45),
        )
    }

    pub fn get_vpn_status(config_name: &str) -> Result<String> {
        let mut command = Self::privileged_command("wg")?;
        command.arg("show").arg(config_name);
        Self::run_command(
            command,
            &format!("query WireGuard interface `{config_name}`"),
        )
    }

    pub fn get_active_vpns() -> Result<Vec<String>> {
        let mut command = Self::privileged_command("wg")?;
        command.args(["show", "interfaces"]);
        let output = Self::run_command(command, "list active WireGuard interfaces")?;
        Ok(Self::parse_interface_names(&output))
    }

    pub fn check_interface_exists(config_name: &str) -> Result<bool> {
        Ok(Self::get_active_vpns()?
            .iter()
            .any(|interface| interface == config_name))
    }

    /// Atomically install in-memory configuration contents with private-key-safe permissions.
    ///
    /// The privileged process only receives trusted bytes over stdin; it never re-opens the
    /// original Downloads path. Publication uses a same-directory hard link, so creation is
    /// atomic and fails if a target with the requested name already exists.
    pub fn install_config(contents: &[u8], target: &Path) -> Result<()> {
        let parent = target
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let file_name = target
            .file_name()
            .ok_or_else(|| {
                anyhow!(
                    "configuration target has no file name: {}",
                    target.display()
                )
            })?
            .to_string_lossy();

        let mut create_dir = Self::privileged_command("install")?;
        create_dir.args(["-d", "-m", "700", "--"]).arg(parent);
        Self::run_command(
            create_dir,
            &format!(
                "prepare WireGuard configuration directory `{}`",
                parent.display()
            ),
        )?;

        let template = parent.join(format!(".{file_name}.wireguard-tui.XXXXXXXXXX"));
        let mut mktemp = Self::privileged_command("mktemp")?;
        mktemp.arg("--").arg(&template);
        let temp_output = Self::run_command(
            mktemp,
            &format!(
                "create a temporary configuration beside `{}`",
                target.display()
            ),
        )?;
        let temp = Self::validated_temp_path(&temp_output, parent, target)?;

        let transaction = (|| -> Result<()> {
            let mut tee = Self::privileged_command("tee")?;
            tee.arg("--").arg(&temp);
            Self::run_command_with_input(
                tee,
                contents,
                &format!(
                    "write temporary WireGuard configuration `{}`",
                    temp.display()
                ),
            )?;

            let mut chmod = Self::privileged_command("chmod")?;
            chmod.args(["600", "--"]).arg(&temp);
            Self::run_command(
                chmod,
                &format!(
                    "secure temporary WireGuard configuration `{}`",
                    temp.display()
                ),
            )?;

            // `link(2)` (via `ln`) is the portable no-clobber primitive available across the
            // privilege boundary: it atomically creates `target` and returns EEXIST instead of
            // replacing an existing configuration. Both paths share a directory, so this cannot
            // cross filesystems.
            let mut publish = Self::privileged_command("ln")?;
            publish.arg("--").arg(&temp).arg(target);
            Self::run_command(
                publish,
                &format!(
                    "atomically install WireGuard configuration `{}`",
                    target.display()
                ),
            )?;

            Self::remove_temporary_file(&temp)?;
            Ok(())
        })();

        if let Err(error) = transaction {
            return match Self::remove_temporary_file(&temp) {
                Ok(()) => Err(error),
                Err(cleanup_error) => Err(anyhow!(
                    "{error:#}; additionally failed to clean temporary file `{}`: {cleanup_error:#}",
                    temp.display()
                )),
            };
        }

        Ok(())
    }

    pub fn remove_config(target: &Path) -> Result<()> {
        let config_name = Self::config_name_from_path(target)?;
        if Self::get_active_vpns()?
            .iter()
            .any(|interface| interface == &config_name)
        {
            return Err(anyhow!(
                "refusing to remove active WireGuard configuration `{config_name}`"
            ));
        }

        let mut command = Self::privileged_command("rm")?;
        command.arg("--").arg(target);
        Self::run_command(
            command,
            &format!("remove WireGuard configuration `{}`", target.display()),
        )?;
        Ok(())
    }

    pub fn read_file(path: &Path) -> Result<String> {
        // Redaction happens inside the privileged process. Private material therefore never
        // crosses back into the unprivileged TUI process through captured stdout.
        let mut command = Self::privileged_command("sed")?;
        command
            .args(["-E", "-e", SECRET_REDACTION_EXPRESSION, "--"])
            .arg(path);
        Self::run_command(command, &format!("read `{}`", path.display()))
    }

    /// Ensure a config cannot be changed by the invoking user between validation
    /// and `wg-quick` reopening it. Every component is inspected without
    /// following the component itself when it is a symlink (`stat` default).
    pub fn ensure_secure_config_path(path: &Path) -> Result<()> {
        if !path.is_absolute() {
            return Err(anyhow!(
                "WireGuard configuration path must be absolute: {}",
                path.display()
            ));
        }

        Self::inspect_secure_path_component(path, true)?;
        let mut ancestor = path.parent();
        while let Some(directory) = ancestor {
            Self::inspect_secure_path_component(directory, false)?;
            ancestor = directory.parent();
        }
        Self::validate_secret_syntax(path)?;
        Ok(())
    }

    fn validate_secret_syntax(path: &Path) -> Result<()> {
        let mut command = Self::privileged_command("awk")?;
        command.arg(SECRET_VALIDATION_AWK).arg(path);
        Self::run_command(
            command,
            &format!(
                "validate private-key syntax in WireGuard config `{}`",
                path.display()
            ),
        )?;
        Ok(())
    }

    fn inspect_secure_path_component(path: &Path, config_file: bool) -> Result<()> {
        let mut command = Self::privileged_command("stat")?;
        command.args(["-c", "%u\t%a\t%F", "--"]).arg(path);
        let output = Self::run_command(
            command,
            &format!("inspect security metadata for `{}`", path.display()),
        )?;
        let (uid, mode, kind) = Self::parse_stat_metadata(&output)?;

        if uid != 0 {
            return Err(anyhow!("`{}` is not owned by root", path.display()));
        }
        if mode & 0o022 != 0 {
            return Err(anyhow!(
                "`{}` is writable by group or other users",
                path.display()
            ));
        }
        if config_file {
            if kind != "regular file" {
                return Err(anyhow!(
                    "WireGuard config `{}` is not a regular file",
                    path.display()
                ));
            }
            if !matches!(mode, 0o400 | 0o600) {
                return Err(anyhow!(
                    "WireGuard config `{}` must have root-only read/write permissions (0400 or 0600)",
                    path.display()
                ));
            }
        } else if kind != "directory" {
            return Err(anyhow!(
                "WireGuard config ancestor `{}` is not a directory",
                path.display()
            ));
        }

        Ok(())
    }

    fn parse_stat_metadata(output: &str) -> Result<(u32, u32, &str)> {
        let mut fields = output.trim().splitn(3, '\t');
        let uid = fields
            .next()
            .ok_or_else(|| anyhow!("stat output omitted owner"))?
            .parse::<u32>()
            .context("stat returned an invalid owner uid")?;
        let mode = u32::from_str_radix(
            fields
                .next()
                .ok_or_else(|| anyhow!("stat output omitted permissions"))?,
            8,
        )
        .context("stat returned invalid permissions")?;
        let kind = fields
            .next()
            .ok_or_else(|| anyhow!("stat output omitted file type"))?;
        Ok((uid, mode, kind))
    }

    pub fn list_config_names(dir: &Path) -> Result<Vec<String>> {
        let mut command = Self::privileged_command("find")?;
        command
            .arg(dir)
            .args(["-maxdepth", "1", "-type", "f", "-name", "*.conf", "-print0"]);
        let output = Self::run_command(
            command,
            &format!("list WireGuard configurations in `{}`", dir.display()),
        )?;
        Ok(Self::parse_config_names(&output))
    }

    fn privileged_command(program: &str) -> Result<Command> {
        let executable = Self::resolve_trusted_executable(program)?;
        if Self::running_as_root() {
            let mut command = Command::new(executable);
            Self::configure_environment(&mut command)?;
            Ok(command)
        } else {
            let sudo = Self::resolve_trusted_executable("sudo")?;
            let mut command = Command::new(sudo);
            Self::configure_environment(&mut command)?;
            command.args(["-n", "--"]).arg(executable);
            Ok(command)
        }
    }

    fn configure_environment(command: &mut Command) -> Result<()> {
        let path = Self::trusted_environment_path()?;
        command.env_clear().env("PATH", path).env("LC_ALL", "C");
        Ok(())
    }

    fn trusted_environment_path() -> Result<OsString> {
        let mut candidates: Vec<PathBuf> = PRIVILEGED_PATH_DIRS.iter().map(PathBuf::from).collect();
        if let Some(path) = env::var_os("PATH") {
            candidates.extend(env::split_paths(&path));
        }

        let mut trusted = Vec::new();
        for candidate in candidates {
            let Ok(canonical) = fs::canonicalize(candidate) else {
                continue;
            };
            if Self::is_trusted_directory_path(&canonical)
                && !trusted.iter().any(|existing| existing == &canonical)
            {
                trusted.push(canonical);
            }
        }
        if trusted.is_empty() {
            return Err(anyhow!(
                "no root-owned, non-writable directory is available for the privileged PATH"
            ));
        }
        env::join_paths(trusted).context("failed to construct the privileged PATH")
    }

    fn run_command(command: Command, action: &str) -> Result<String> {
        Self::run_command_with_timeout(command, action, Duration::from_secs(10))
    }

    fn run_command_with_timeout(
        mut command: Command,
        action: &str,
        timeout: Duration,
    ) -> Result<String> {
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        command.process_group(0);

        let mut child = command
            .spawn()
            .with_context(|| format!("failed to {action}"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("failed to capture stdout while attempting to {action}"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow!("failed to capture stderr while attempting to {action}"))?;
        let readers_done = Arc::new(AtomicBool::new(false));
        let stdout_done = Arc::clone(&readers_done);
        let stderr_done = Arc::clone(&readers_done);
        let stdout_reader = thread::spawn(move || Self::read_stream(stdout, stdout_done));
        let stderr_reader = thread::spawn(move || Self::read_stream(stderr, stderr_done));

        let deadline = Instant::now() + timeout;
        let mut timed_out = false;
        let status_result = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Ok(status),
                Ok(None) if Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(20));
                }
                Ok(None) => {
                    timed_out = true;
                    Self::terminate_process_tree(&mut child);
                    break child.wait().with_context(|| {
                        format!("failed to reap timed-out process while attempting to {action}")
                    });
                }
                Err(error) => {
                    Self::terminate_process_tree(&mut child);
                    let _ = child.wait();
                    break Err(error)
                        .with_context(|| format!("failed to wait while attempting to {action}"));
                }
            }
        };
        readers_done.store(true, Ordering::Release);

        let stdout = Self::join_reader(stdout_reader, "stdout", action)?;
        let stderr = Self::join_reader(stderr_reader, "stderr", action)?;
        let status = status_result?;
        let output = Output {
            status,
            stdout,
            stderr,
        };

        if timed_out {
            let stdout = Self::redact_command_diagnostics(&String::from_utf8_lossy(&output.stdout));
            let stderr = Self::redact_command_diagnostics(&String::from_utf8_lossy(&output.stderr));
            return Err(anyhow!(
                "timed out after {:.1}s while attempting to {action}: {}",
                timeout.as_secs_f32(),
                Self::failure_detail(&stdout, &stderr)
            ));
        }

        Self::successful_stdout(output, action)
    }

    fn read_stream(
        mut stream: impl Read + AsRawFd,
        command_done: Arc<AtomicBool>,
    ) -> std::io::Result<Vec<u8>> {
        #[cfg(unix)]
        {
            let descriptor = stream.as_raw_fd();
            // SAFETY: `descriptor` belongs to the live pipe object. `F_GETFL` and
            // `F_SETFL` do not take pointer arguments, and failure is propagated.
            let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
            if flags < 0
                || unsafe { libc::fcntl(descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
            {
                return Err(std::io::Error::last_os_error());
            }
        }

        let mut contents = Vec::new();
        let mut buffer = [0_u8; 8192];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) => return Ok(contents),
                Ok(count) => {
                    if contents.len().saturating_add(count) > MAX_COMMAND_OUTPUT_BYTES {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "command output exceeded the 1 MiB safety limit",
                        ));
                    }
                    contents.extend_from_slice(&buffer[..count]);
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if command_done.load(Ordering::Acquire) {
                        return Ok(contents);
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => return Err(error),
            }
        }
    }

    fn join_reader(
        reader: thread::JoinHandle<std::io::Result<Vec<u8>>>,
        stream: &str,
        action: &str,
    ) -> Result<Vec<u8>> {
        reader
            .join()
            .map_err(|_| anyhow!("{stream} reader panicked while attempting to {action}"))?
            .with_context(|| {
                format!("failed to read command {stream} while attempting to {action}")
            })
    }

    fn terminate_process_tree(child: &mut std::process::Child) {
        #[cfg(unix)]
        {
            // The command is placed in its own process group above. Killing the group prevents a
            // hanging wg-quick hook from surviving after its parent command reaches the timeout.
            let process_group = -(child.id() as i32);
            // SAFETY: `kill` is called with a process-group id created for this child and a valid
            // signal constant. Failure is harmless and followed by `Child::kill` as a fallback.
            let _ = unsafe { libc::kill(process_group, libc::SIGKILL) };
        }
        let _ = child.kill();
    }

    fn run_command_with_input(mut command: Command, input: &[u8], action: &str) -> Result<String> {
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        command.process_group(0);
        let mut child = command
            .spawn()
            .with_context(|| format!("failed to {action}"))?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("failed to open command stdin while attempting to {action}"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow!("failed to capture stderr while attempting to {action}"))?;
        let owned_input = input.to_vec();
        let input_writer = thread::spawn(move || stdin.write_all(&owned_input));
        let reader_done = Arc::new(AtomicBool::new(false));
        let stderr_done = Arc::clone(&reader_done);
        let stderr_reader = thread::spawn(move || Self::read_stream(stderr, stderr_done));

        let timeout = Duration::from_secs(10);
        let deadline = Instant::now() + timeout;
        let mut timed_out = false;
        let status_result = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Ok(status),
                Ok(None) if Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(20));
                }
                Ok(None) => {
                    timed_out = true;
                    Self::terminate_process_tree(&mut child);
                    break child.wait().with_context(|| {
                        format!("failed to reap timed-out process while attempting to {action}")
                    });
                }
                Err(error) => {
                    Self::terminate_process_tree(&mut child);
                    let _ = child.wait();
                    break Err(error)
                        .with_context(|| format!("failed to wait while attempting to {action}"));
                }
            }
        };
        reader_done.store(true, Ordering::Release);

        let write_result = input_writer
            .join()
            .map_err(|_| anyhow!("stdin writer panicked while attempting to {action}"))?
            .with_context(|| {
                format!("failed to stream configuration bytes while attempting to {action}")
            });
        let stderr = Self::join_reader(stderr_reader, "stderr", action)?;
        let status = status_result?;
        let output = Output {
            status,
            stdout: Vec::new(),
            stderr,
        };

        if timed_out {
            let stderr = Self::redact_command_diagnostics(&String::from_utf8_lossy(&output.stderr));
            return Err(anyhow!(
                "timed out after {:.1}s while attempting to {action}: {}",
                timeout.as_secs_f32(),
                Self::failure_detail("", &stderr)
            ));
        }
        let command_result = Self::successful_stdout(output, action);

        match (write_result, command_result) {
            (Ok(()), result) => result,
            (Err(write_error), Ok(_)) => Err(write_error),
            (Err(write_error), Err(command_error)) => Err(anyhow!(
                "{write_error:#}; command also failed: {command_error:#}"
            )),
        }
    }

    fn remove_temporary_file(path: &Path) -> Result<()> {
        let mut command = Self::privileged_command("rm")?;
        command.args(["-f", "--"]).arg(path);
        Self::run_command(
            command,
            &format!("clean temporary configuration `{}`", path.display()),
        )?;
        Ok(())
    }

    fn successful_stdout(output: Output, action: &str) -> Result<String> {
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        if output.status.success() {
            return Ok(stdout);
        }

        let stdout = Self::redact_command_diagnostics(&stdout);
        let stderr = Self::redact_command_diagnostics(&String::from_utf8_lossy(&output.stderr));
        Err(anyhow!(
            "failed to {action} ({}): {}",
            output.status,
            Self::failure_detail(&stdout, &stderr)
        ))
    }

    fn failure_detail(stdout: &str, stderr: &str) -> String {
        let stdout = stdout.trim();
        let stderr = stderr.trim();
        match (stderr.is_empty(), stdout.is_empty()) {
            (false, false) => format!("stderr: {stderr}\nstdout: {stdout}"),
            (false, true) => format!("stderr: {stderr}"),
            (true, false) => format!("stdout: {stdout}"),
            (true, true) => "command produced no output".to_string(),
        }
    }

    fn redact_command_diagnostics(value: &str) -> String {
        value
            .lines()
            .map(|line| {
                let lowercase = line.to_ascii_lowercase();
                if lowercase.contains("key is not the correct length or format") {
                    "WireGuard rejected <redacted key material>".to_string()
                } else if line.contains('=')
                    && [
                        "privatekey",
                        "presharedkey",
                        "password",
                        "passwd",
                        "credential",
                        "token",
                        "secret",
                        "api_key",
                    ]
                    .iter()
                    .any(|marker| lowercase.contains(marker))
                {
                    let (key, _) = line.split_once('=').expect("assignment checked");
                    format!("{key}= <redacted>")
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn parse_interface_names(output: &str) -> Vec<String> {
        output.split_whitespace().map(ToOwned::to_owned).collect()
    }

    fn config_name_from_path(path: &Path) -> Result<String> {
        if path.extension().and_then(|extension| extension.to_str()) != Some("conf") {
            return Err(anyhow!(
                "WireGuard configuration must end in `.conf`: {}",
                path.display()
            ));
        }

        path.file_stem()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| {
                anyhow!(
                    "WireGuard configuration has an invalid file name: {}",
                    path.display()
                )
            })
    }

    fn parse_config_names(output: &str) -> Vec<String> {
        let mut names: Vec<String> = output
            .split('\0')
            .filter(|path| !path.is_empty())
            .filter_map(|path| Path::new(path).file_name())
            .filter_map(|name| name.to_str())
            .filter_map(|name| name.strip_suffix(".conf"))
            .map(ToOwned::to_owned)
            .collect();
        names.sort();
        names.dedup();
        names
    }

    fn validated_temp_path(output: &str, parent: &Path, target: &Path) -> Result<PathBuf> {
        let value = output.trim();
        if value.is_empty() || value.contains(['\n', '\r']) {
            return Err(anyhow!("mktemp returned an invalid path: {output:?}"));
        }

        let path = PathBuf::from(value);
        if path == target || path.parent() != Some(parent) {
            return Err(anyhow!(
                "mktemp returned a path outside the target directory: {}",
                path.display()
            ));
        }
        Ok(path)
    }

    fn command_exists(program: &str) -> bool {
        Self::resolve_trusted_executable(program).is_ok()
    }

    /// Resolve a command to an immutable-enough, root-controlled absolute executable path.
    ///
    /// Fixed system directories are searched before PATH. PATH remains useful for unusual but
    /// correctly secured installations; every resolved file and every one of its ancestor
    /// directories must be owned by root and not writable by group or other users.
    fn resolve_trusted_executable(program: &str) -> Result<PathBuf> {
        let program_path = Path::new(program);
        if program.is_empty()
            || program_path.is_absolute()
            || program_path.components().count() != 1
            || program == "."
            || program == ".."
        {
            return Err(anyhow!("invalid executable name: {program:?}"));
        }

        let mut directories: Vec<PathBuf> =
            TRUSTED_EXECUTABLE_DIRS.iter().map(PathBuf::from).collect();
        if let Some(path) = env::var_os("PATH") {
            for directory in env::split_paths(&path) {
                if !directories.iter().any(|existing| existing == &directory) {
                    directories.push(directory);
                }
            }
        }

        for directory in directories {
            let candidate = directory.join(program);
            let Ok(canonical) = fs::canonicalize(&candidate) else {
                continue;
            };
            if canonical.is_absolute() && Self::is_trusted_executable_path(&canonical) {
                return Ok(canonical);
            }
        }

        Err(anyhow!(
            "trusted root-owned executable `{program}` was not found"
        ))
    }

    fn is_trusted_executable_path(path: &Path) -> bool {
        let Ok(metadata) = fs::metadata(path) else {
            return false;
        };
        if !metadata.is_file() || !Self::has_trusted_metadata(&metadata, true) {
            return false;
        }

        let mut ancestor = path.parent();
        while let Some(directory) = ancestor {
            let Ok(metadata) = fs::metadata(directory) else {
                return false;
            };
            if !metadata.is_dir() || !Self::has_trusted_metadata(&metadata, false) {
                return false;
            }
            ancestor = directory.parent();
        }

        true
    }

    fn is_trusted_directory_path(path: &Path) -> bool {
        let mut ancestor = Some(path);
        while let Some(directory) = ancestor {
            let Ok(metadata) = fs::metadata(directory) else {
                return false;
            };
            if !metadata.is_dir() || !Self::has_trusted_metadata(&metadata, false) {
                return false;
            }
            ancestor = directory.parent();
        }
        true
    }

    #[cfg(unix)]
    fn has_trusted_metadata(metadata: &fs::Metadata, executable: bool) -> bool {
        Self::has_trusted_owner_and_mode(metadata.uid(), metadata.permissions().mode(), executable)
    }

    #[cfg(unix)]
    fn has_trusted_owner_and_mode(uid: u32, mode: u32, executable: bool) -> bool {
        uid == 0 && mode & 0o022 == 0 && (!executable || mode & 0o111 != 0)
    }

    #[cfg(not(unix))]
    fn has_trusted_metadata(_metadata: &fs::Metadata, _executable: bool) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::CommandExecutor;
    use std::io::Write;
    use std::path::Path;
    use std::process::{Command, Stdio};

    #[test]
    fn resolves_only_simple_names_to_trusted_absolute_paths() {
        let executable = CommandExecutor::resolve_trusted_executable("sh").unwrap();
        assert!(executable.is_absolute());
        assert!(CommandExecutor::is_trusted_executable_path(&executable));

        assert!(CommandExecutor::resolve_trusted_executable("/bin/sh").is_err());
        assert!(CommandExecutor::resolve_trusted_executable("../bin/sh").is_err());
        assert!(CommandExecutor::resolve_trusted_executable("").is_err());
    }

    #[test]
    fn rejects_untrusted_owners_and_writable_modes() {
        assert!(CommandExecutor::has_trusted_owner_and_mode(
            0, 0o100755, true
        ));
        assert!(!CommandExecutor::has_trusted_owner_and_mode(
            1000, 0o100755, true
        ));
        assert!(!CommandExecutor::has_trusted_owner_and_mode(
            0, 0o100775, true
        ));
        assert!(!CommandExecutor::has_trusted_owner_and_mode(
            0, 0o100644, true
        ));
        assert!(CommandExecutor::has_trusted_owner_and_mode(
            0, 0o040755, false
        ));
    }

    #[test]
    fn parses_all_active_interfaces_in_order() {
        assert_eq!(
            CommandExecutor::parse_interface_names("wg-home wg-work\nwg-test\n"),
            ["wg-home", "wg-work", "wg-test"]
        );
    }

    #[test]
    fn parses_sorted_unique_config_names_from_find_output() {
        let output = "/etc/wireguard/zeta.conf\0/etc/wireguard/alpha.conf\0\
                      /etc/wireguard/alpha.conf\0/etc/wireguard/readme.txt\0";
        assert_eq!(
            CommandExecutor::parse_config_names(output),
            ["alpha", "zeta"]
        );
    }

    #[test]
    fn command_failure_keeps_stderr_and_stdout() {
        let detail = CommandExecutor::failure_detail(
            "partial command output\n",
            "sudo: a password is required\n",
        );
        assert!(detail.contains("stderr: sudo: a password is required"));
        assert!(detail.contains("stdout: partial command output"));
    }

    #[test]
    fn command_diagnostics_redact_invalid_and_assigned_keys() {
        let diagnostic = CommandExecutor::redact_command_diagnostics(
            "Key is not the correct length or format: `top-secret'\nPrivateKey = another-secret\nPassword=provider-secret\naccess_token=api-secret\nordinary failure",
        );
        assert!(!diagnostic.contains("top-secret"));
        assert!(!diagnostic.contains("another-secret"));
        assert!(!diagnostic.contains("provider-secret"));
        assert!(!diagnostic.contains("api-secret"));
        assert!(diagnostic.contains("<redacted key material>"));
        assert!(diagnostic.contains("Password= <redacted>"));
        assert!(diagnostic.contains("ordinary failure"));
    }

    #[test]
    fn validates_same_directory_temporary_paths() {
        let path = CommandExecutor::validated_temp_path(
            "/etc/wireguard/.wg0.conf.wireguard-tui.a1b2c3\n",
            std::path::Path::new("/etc/wireguard"),
            std::path::Path::new("/etc/wireguard/wg0.conf"),
        )
        .unwrap();
        assert_eq!(
            path,
            std::path::Path::new("/etc/wireguard/.wg0.conf.wireguard-tui.a1b2c3")
        );

        assert!(CommandExecutor::validated_temp_path(
            "/tmp/untrusted",
            std::path::Path::new("/etc/wireguard"),
            std::path::Path::new("/etc/wireguard/wg0.conf"),
        )
        .is_err());
    }

    #[test]
    fn derives_interface_name_only_from_conf_paths() {
        assert_eq!(
            CommandExecutor::config_name_from_path(Path::new("/etc/wireguard/wg-home.conf"))
                .unwrap(),
            "wg-home"
        );
        assert!(
            CommandExecutor::config_name_from_path(Path::new("/etc/wireguard/wg-home.txt"))
                .is_err()
        );
        assert!(CommandExecutor::config_name_from_path(Path::new("/etc/wireguard/.conf")).is_err());
    }

    #[test]
    fn parses_security_metadata_without_file_content() {
        assert_eq!(
            CommandExecutor::parse_stat_metadata("0\t600\tregular file\n").unwrap(),
            (0, 0o600, "regular file")
        );
        assert_eq!(
            CommandExecutor::parse_stat_metadata("0\t755\tdirectory\n").unwrap(),
            (0, 0o755, "directory")
        );
        assert!(CommandExecutor::parse_stat_metadata("root 600 file").is_err());
    }

    #[test]
    fn privileged_redaction_removes_both_wireguard_secret_types() {
        let sed = CommandExecutor::resolve_trusted_executable("sed").unwrap();
        let mut child = Command::new(sed)
            .args(["-E", "-e", super::SECRET_REDACTION_EXPRESSION])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(
                b"[Interface]\nPrivateKey = secret-one\n[Peer]\nPresharedKey=secret-two\nPublicKey = public\n",
            )
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        let output = String::from_utf8(output.stdout).unwrap();
        assert!(!output.contains("secret-one"));
        assert!(!output.contains("secret-two"));
        assert!(output.contains("PrivateKey = <redacted>"));
        assert!(output.contains("PresharedKey=<redacted>"));
        assert!(output.contains("PublicKey = public"));
    }

    #[test]
    fn privileged_secret_validator_never_prints_key_material() {
        let awk = CommandExecutor::resolve_trusted_executable("awk").unwrap();
        for (config, expected_success) in [
            (
                "[Interface]\nPrivateKey = AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=\n",
                true,
            ),
            (
                "[Interface]\nPrivateKey = invalid-secret-that-must-not-be-echoed\n",
                false,
            ),
            (
                "[Peer]\nPresharedKey = AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=\n",
                false,
            ),
        ] {
            let mut child = Command::new(&awk)
                .arg(super::SECRET_VALIDATION_AWK)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            child
                .stdin
                .take()
                .unwrap()
                .write_all(config.as_bytes())
                .unwrap();
            let output = child.wait_with_output().unwrap();
            assert_eq!(output.status.success(), expected_success);
            assert!(output.stdout.is_empty());
            assert!(output.stderr.is_empty());
        }
    }
}
