# Upgrade rounds: parser, filesystem, and diagnostics

This ledger records the twenty-two independently reviewable hardening increments in
the current upgrade batch.  The limits are deliberately conservative and are
checked before data is copied into long-lived application state.

1. **Bounded grammar input.** Configuration validation rejects inputs over 64
   KiB, over 4096 lines, or over 256 peer sections.
2. **Supported-key allowlist.** Interface and peer keys are matched against the
   documented `wg`/`wg-quick` vocabulary; unsupported diagnostics never echo
   the attacker-controlled key text.
3. **Singleton consistency.** Singleton interface/peer fields, including
   `FwMark` and `SaveConfig`, reject ambiguous duplicates while documented
   repeatable fields remain repeatable.
4. **MTU validation.** `MTU` is parsed completely and restricted to the usable
   1..=65535 range.
5. **Routing-table validation.** `Table` accepts `auto`, `off`, numeric table
   identifiers, and bounded safe table names without shell metacharacters.
6. **Mark and persistence validation.** `FwMark` accepts `off`, decimal, or
   hexadecimal `u32`; `SaveConfig` accepts only the documented booleans.
7. **IP-list structure.** `Address`/`AllowedIPs` reject empty list elements,
   repeated slash separators, and missing address or prefix components while
   retaining bare-address and repeated-field compatibility.
8. **Peer identity uniqueness.** Duplicate peer public keys across sections are
   rejected without including key material in the error.
9. **Details aggregation.** Repeatable `Address`, `DNS`, and first-peer
   `AllowedIPs` values are accumulated consistently, without mixing peers.
10. **Exact secret redaction.** Configuration display redacts private and
    preshared keys while preserving CRLF and final-newline shape.
11. **Bounded installed reads.** Installed configurations are opened with
    no-follow/nonblocking safeguards, must be regular files, and are read under
    a 64 KiB cap with before/after identity checks.
12. **Private application state.** Existing application config is size-,
    ownership-, and link-count checked and normalized to mode 0600.
13. **Crash-aware migration.** Default-config migration uses a same-directory
    create-new temporary file, file sync, atomic rename, and parent sync.
14. **Privileged inspection budget.** Secure-path inspection reports and
    enforces installed configuration size before privileged reads.
15. **Active-interface contract.** `wg` interface output is name-validated,
    stable-deduplicated, and capped at 256 entries.
16. **Status identity binding.** Parsed status must name the requested valid
    interface and cannot contain duplicate interface records.
17. **Status resource budget.** Status parsing caps total bytes, line bytes,
    line count, and peer count and rejects control characters in values.
18. **Diagnostic secret safety.** Command failures redact sensitive assignments
    written with equals, colon, or whitespace separators while retaining useful
    nonsensitive endpoint diagnostics.
19. **Single startup warning.** Startup warnings enter the error channel once,
    avoiding duplicate user-visible state transitions.
20. **Bounded import discovery.** Downloads scanning fails with an actionable
    error after 4096 directory entries or 256 eligible configurations.
21. **Installed listing contract.** Privileged `find` output is reduced to valid
    interface stems, stable-deduplicated, sorted, and capped at 256 names before
    the list enters application state.
22. **Active-interface listing contract.** Parsed `wg` interface names are
    validated, stable-deduplicated, sorted, and capped at 256 entries before
    they enter application state, matching the installed-configuration listing.
23. **Listing control-character contract.** Privileged `wg` and `find` listing
    lines reject embedded control characters before interface validation or
    stem extraction, matching the status parser's diagnostic safety rule.
24. **Status label control-character contract.** `wg show` status parsing now
    rejects control characters in field labels as well as values, closing a
    diagnostic echo gap where poisoned label text could reach the UI.
25. **Status public-key presence contract.** Interface-scope `public key:`
    lines must carry non-empty values before they enter parsed status, closing
    a gap where a malformed `wg show` dump could advertise an empty key.
26. **Peer public-key presence contract.** `peer:` lines must carry non-empty
    values before they enter parsed status, closing the same gap for peer keys.
27. **Interface name presence contract.** `interface:` lines must carry non-empty
    values before they enter parsed status, closing a gap where a malformed
    `wg show` dump could advertise an empty interface name.
28. **Listening-port presence contract.** Interface-scope `listening port:` lines
    must carry non-empty values before they enter parsed status, closing a gap
    where a malformed `wg show` dump could advertise an empty port.

Regression coverage lives beside the relevant modules plus
`tests/test_import.rs`; the final acceptance commands are `cargo test`,
`cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --check`,
and `git diff --check`.
