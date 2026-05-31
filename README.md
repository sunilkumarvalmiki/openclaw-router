# OpenClaw Router

An intelligent LLM routing gateway for sending model requests through cost-aware, policy-aware, and
provider-aware routing paths with dashboard and infrastructure support.

## Overview

Active Rust/TypeScript project. It includes a gateway, dashboard, Docker assets, documentation,
scripts, and license files.

## What This Repository Contains

- Rust gateway code under `gateway/`.
- Dashboard application under `dashboard/`.
- Docker and deployment assets under `docker/`.
- Operational and architecture documentation under `docs/`.
- Scripts for local development and maintenance.

## Who This Is For

- LLM platform engineers
- AI gateway builders
- Rust backend developers
- Cost-optimization and routing maintainers

## Repository Structure

| Path | Purpose |
|------|---------|
| `gateway/` | LLM routing gateway implementation. |
| `dashboard/` | Administrative or monitoring dashboard. |
| `docker/` | Container and compose assets. |
| `docs/` | Architecture, operations, and usage documentation. |
| `scripts/` | Maintenance and local workflow scripts. |
| `LICENSE` | Project license. |

## Getting Started

- Install Rust and Node.js according to the gateway and dashboard requirements.
- Use the Docker assets when you need an isolated local stack.
- Read `docs/` before changing routing behavior or provider configuration.

## Common Workflows

- Keep provider credentials in environment variables or secret stores.
- Validate routing changes with representative provider and policy scenarios before deployment.

## Quality, Security, And Maintenance Notes

- Do not log prompts, secrets, provider tokens, or customer data unless a sanitized testing fixture is explicitly intended.
- Document cost-saving claims with reproducible benchmarks or assumptions.

## Current Documentation State

This README was rewritten to make the repository purpose, structure, setup path, and safety
expectations clear to a new reader. If implementation details change, update this file in the same
change so the GitHub landing page stays accurate.

Last documentation refresh: 2026-05-31.
