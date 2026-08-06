# Task Triage

Write a resolved objective with end state, hard constraints, exclusions, deliverables, environment, and evidence required.

Classify ambiguity as:

- derivable: inspect; do not ask;
- reversible: choose the narrowest safe default;
- product-defining: ask only when outcomes materially differ;
- safety-critical: stop or clarify.

Score 0–3 for unfamiliarity, breadth, API instability, data-loss risk, security, runtime observability, test weakness, dependency complexity, migration complexity, and external-fact freshness.

- 0–5: S
- 6–11: M
- 12–20: L
- 21+: XL

Acceptance gates must be falsifiable and include negative gates such as no unintended files, no incompatible duplicate dependency, no disabled required feature, and no unsupported success claim.
