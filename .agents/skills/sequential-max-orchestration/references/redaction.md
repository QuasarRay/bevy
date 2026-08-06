# Evidence Redaction

Before storing logs or handoffs:

- remove tokens, passwords, cookies, private keys, authorization headers, connection strings, and personal data not required for proof;
- replace secrets with stable labels such as `<REDACTED_TOKEN_A>`;
- do not weaken the actual command or test environment merely to produce a cleaner log;
- store the minimal excerpt needed;
- note that redaction occurred;
- never commit runtime evidence by default.
