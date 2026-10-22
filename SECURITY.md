# Security policy

## Supported versions

There are no releases yet, so only the latest commit on `main` gets security fixes. Please check that the
problem still happens there before reporting.

## Reporting a vulnerability

This repository is private, so only its collaborators can see it. Please do not open an issue, pull request or
discussion for a security problem.

Report it privately to the owner, [@izam-mohammed](https://github.com/izam-mohammed), on GitHub.

Helpful details:

- the commit you built and the MCP client you used (Claude Code, Codex or another)
- what an attacker could do, and what they need first (for example text printed into a pane, a crafted pane or
  tab name, or a client started outside herdr)
- the smallest steps or script that show it, run against an isolated `herdr --session` and never your real one
- any fix you have in mind

## What happens next

This is a project with one maintainer, so there are no guaranteed response times. We aim to:

- acknowledge the report
- agree on whether it is a vulnerability and how serious it is
- fix it on `main` and tell you when the fix has landed

## Scope

In scope: the `herdr-mcp` server and every tool it exposes, especially anything that gets around the refusals
in the README (prompting, typing into or closing its own pane, steering herdr from outside it, reaching
herdr's stop, restart or config methods, or treating pane output as instructions), and the documented install
path.

Problems in herdr itself or in the MCP clients should go to those projects; tell us too if herdr-mcp makes them
worse.
