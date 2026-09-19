# Security Policy

## Supported Versions

Only the latest 2.x release is supported.

## Reporting a Vulnerability

Use GitHub's private vulnerability reporting: open the repository's
**Security** tab and choose **"Report a vulnerability"**. Do not open a
public issue for sensitive reports. There is no bug bounty program.

You will receive an acknowledgment within a week; a fix timeline is
communicated after triage.

## Scope

This crate forbids unsafe code; memory safety still depends on the soundness
of its dependencies. Incorrect transforms can affect physical systems. Report
sensitive correctness bugs through the same private channel; use a regular
issue for non-sensitive bugs.
