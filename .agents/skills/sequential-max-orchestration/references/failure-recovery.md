# Failure Recovery

First failure: preserve exact command, isolate the first causal error, check environment, and attempt one bounded correction only when cause is clear.

Use `seqmax_diagnostician` when root cause remains unclear, dependencies interact, failure is nondeterministic, runtime contradicts static evidence, or strategies materially differ.

Default retry budget: one implementer repair, one diagnostic cycle, one remediation, one final verification. A second diagnostic cycle requires genuinely new evidence.

Rollback when the strategy violates a hard constraint, masks the cause, expands beyond architecture, or becomes less verifiable than clean reimplementation.
