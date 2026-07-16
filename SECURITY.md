# Security Policy

## Supported versions

Security fixes are provided for the latest `0.4.x` release. Older development snapshots and generated binaries are not supported.

## Reporting a vulnerability

Please use GitHub's private security advisory flow for this repository. Do not open a public issue containing credentials, private keys, complete WireGuard configurations, command output with secrets, or a working exploit.

Include the affected version, operating system, reproduction steps, impact, and a minimally redacted sample when possible. Remove VPN usernames, passwords, private keys, endpoints tied to a private service, and public IP addresses before sharing logs.

## Credential handling

### Known credential incident in pre-0.4 history

An early repository revision committed provider username and password values in the application template. Those values must be treated as compromised: rotate or revoke them immediately and review the provider account for unexpected sessions or generated profiles. Version 0.4 removes credential handling from the application, but a normal follow-up commit does not erase the values from existing Git history, forks, clones, or caches. A coordinated history rewrite and force-push are still required to remove those historical copies.

The application configuration is stored at:

```text
~/.config/wireguard-tui/config.toml
```

It intentionally contains no provider username or password. On the first successful read, version 0.4 erases all obsolete fields from a legacy application config and replaces them with the credential-free template. Protect the file with user-only permissions:

```bash
chmod 600 ~/.config/wireguard-tui/config.toml
```

Do not run the complete TUI as root. Run `sudo -v` before starting the application as the normal desktop user. Version 0.4 does not install system software automatically.

Never commit application configuration, WireGuard private keys, captured terminal output, screenshots, shell transcripts, or provider credentials. Enter provider credentials only in the authenticated browser page; the TUI does not need them.

### If a credential has been exposed

1. Rotate the VPN password and any exposed WireGuard key material immediately through the provider. Revoke active sessions or old profiles when supported.
2. Remove the secret from the working tree, logs, CI artifacts, screenshots, release archives, caches, and copied test fixtures.
3. Search the full repository history and all refs. A later deletion commit does not remove a secret from earlier commits.
4. If history contains the secret, rewrite it with a tool such as `git filter-repo`, coordinate a force-push, invalidate cached artifacts, and require collaborators to re-clone or carefully clean their local refs.
5. Enable repository secret scanning and review forks, pull-request patches, Actions logs, and release attachments.

History rewriting is disruptive and is not a substitute for rotation. Treat a published credential as compromised even after every known copy is removed.

## Threat model for imported WireGuard configurations

A WireGuard configuration is not merely passive connection data. `wg-quick` supports these directives:

- `PreUp`
- `PostUp`
- `PreDown`
- `PostDown`

Their values are shell commands. When `wg-quick` is invoked through sudo, a malicious hook can execute arbitrary commands with root privileges. Possible impact includes credential theft, persistent system modification, disabling security controls, replacing binaries, or destroying data.

Other fields can also have security impact: `AllowedIPs` and routing-table settings can redirect traffic, DNS settings can influence name resolution, and an attacker-controlled endpoint can observe tunneled traffic metadata.

Only import configurations obtained directly from a trusted provider over an authenticated channel. Before importing, inspect any candidate for hook directives, for example:

```bash
grep -Ein -- '^[[:space:]]*(PreUp|PostUp|PreDown|PostDown)[[:space:]]*=' profile.conf
```

An empty grep result is not proof that a configuration is safe. Review the complete file, its origin, ownership, permissions, routes, DNS, and endpoint.

Version 0.4 rejects import candidates containing any of the four hook directives above, validates their minimum client structure, and summarizes batch failures. It installs validated bytes as root with mode `0600`, without asking a privileged process to reopen the original Downloads path and without replacing an existing target. Before every `wg-quick` action, an installed config must be a root-owned regular file with mode `0400` or `0600`, beneath root-owned ancestors that are not writable by group or other users. These controls are defense in depth rather than a sandbox; future syntax not recognized by the validator remains part of the trust boundary. Never approve an import solely because it passed application checks.

## Privilege boundary

The TUI itself should run as an unprivileged user. Only narrowly scoped WireGuard and configuration operations should cross the sudo boundary. A cached sudo credential grants meaningful local authority; lock the workstation while it is valid and use `sudo -k` when finished if the cache is no longer needed.

The application cannot protect against a malicious local root user, a compromised provider account, a compromised dependency or compiler, or an already modified `wg`/`wg-quick` executable.

## Dependency and build security

Releases and CI should build with the committed `Cargo.lock` and `--locked`. Review dependency changes, keep Rust and system packages current, and do not bypass failing format, Clippy, test, or build checks.
