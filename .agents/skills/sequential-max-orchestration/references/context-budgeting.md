# Context Budgeting

Max reasoning is expensive. Reduce calls and noise, not reasoning quality.

Every spawn packet includes: role, one objective, reason, in-scope paths, out-of-scope work, verified facts, open questions, constraints, allowed mutations, required evidence, exit criteria, handoff format, and no-delegation rule.

Do not paste the entire conversation. Reference large logs and metadata by path, hash, and search terms.

Default handoff limits:

- summary <= 250 words;
- <= 12 findings;
- only load-bearing commands;
- <= 8 unresolved risks;
- exactly one next action.

Before rescanning, inspect the active task's evidence index. Expand scope only when evidence requires it.
