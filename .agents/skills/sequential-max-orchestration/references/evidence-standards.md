# Evidence Standards

Strength order:

1. reproduced runtime behavior;
2. executable test;
3. compiler/type checker/resolver/static analyzer;
4. direct source/manifest/config inspection;
5. versioned primary documentation;
6. maintainer issue/discussion;
7. labeled inference;
8. unsupported assertion.

Negative claims require systematic evidence, such as dependency trees, exhaustive search, or a feature/target test matrix.

Command evidence records working directory, exact command, exit code, relevant output, environment overrides, truncation, and log path.

Runtime evidence distinguishes compiled, started, initialized, exercised, and observed result.

Inference format: evidence, confidence, and what would falsify it.
