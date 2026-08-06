# Handoff Protocol

Every child returns:

1. Status: `COMPLETE`, `PARTIAL`, `BLOCKED`, or `NO_CHANGE_REQUIRED`.
2. Objective completed.
3. Verified findings with evidence.
4. Files changed, or `none`.
5. Commands, exit codes, and log paths.
6. Remaining assumptions.
7. Ranked risks.
8. Acceptance criteria: `PASS`, `FAIL`, `NOT TESTED`, or `N/A`.
9. Exactly one recommended next action.
10. Attestation: `I did not spawn or delegate to another agent.`

The parent validates at least one load-bearing finding, every destructive/architectural claim, claimed test results, and important negative claims.
