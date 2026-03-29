module.exports = {
  extends: ["@commitlint/config-conventional"],
  rules: {
    "type-enum": [
      2,
      "always",
      [
        "feat",
        "fix",
        "perf",
        "test",
        "docs",
        "ci",
        "refactor",
        "style",
        "build",
        "chore",
      ],
    ],
    "body-max-line-length": [0, "always", Infinity],
  },
};
