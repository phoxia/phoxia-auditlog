<!-- PHOXIA-DEVKIT:START -->
# Phoxia open-source development profile

- Project instructions, accepted RFCs, schemas and tests are authoritative.
- Products own their domains and private persistence.
- Cross-domain integration uses versioned APIs, commands or Pulse events.
- Pulse consumers are idempotent and projections are rebuildable.
- Events record completed facts. Commands request intent.
- Security, privacy, accessibility, cost and observability are architecture requirements.
- Open-source readiness, community documentation and license compatibility are part of completion.
- Use `Phoxia • Page` for page titles.
- Avoid em dashes in normal product copy.
- Prefer `web`, `api`, `mobile`, `worker`, `cli` and `docs` for application names.

## Project context

- Name: phoxia-auditlog
- Purpose: Axum middleware for automatic PostgreSQL-backed audit logging.
- Repository visibility: public
- Documentation visibility: public
- Distribution model: open-source

## DevKit workflows

- Inspect `.phoxia/project.yaml` before material changes.
- Use the vendored Phoxia skills for development, project, release, documentation, UI and handoff workflows.
- Preserve user-owned instructions outside the managed block.
- A public version manifest change requires a changelog entry.
- A user-facing contract or behavior change requires documentation.
- Use a Feature Record for a significant capability, an ADR for an important local technical decision, and RFC analysis for cross-domain or governance changes.
- Run `$phoxia-devkit` to review DevKit configuration.
<!-- PHOXIA-DEVKIT:END -->
