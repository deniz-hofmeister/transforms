# Security Policy

## Supported Versions

| Version              | Supported |
|----------------------|-----------|
| latest 2.x release   | Yes       |
| < 2.0                | No        |

## Reporting a Vulnerability

Use GitHub's private vulnerability reporting: open the repository's
**Security** tab and choose **"Report a vulnerability"**. Do not open a
public issue for sensitive reports. There is no bug bounty program.

You will receive an acknowledgment within a week; a fix timeline is
communicated after triage.

## Scope

This is a coordinate transform library. Memory safety is enforced via
`#![forbid(unsafe_code)]`, so classic memory-corruption vulnerabilities are
ruled out by construction. This code positions robots, however, and the worst
failure mode is not a crash — it is a plausible-looking wrong answer returned
silently. Correctness bugs that produce wrong transforms in safety-relevant
systems are therefore treated with security-level seriousness: if such a bug
is sensitive, report it through the same private channel; otherwise a regular
bug report is fine.
