module.exports = {
  extends: ["@commitlint/config-conventional"],
  ignores: [(message) => message.includes("Merge branch")],
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
    "header-max-length": [0, "always", 150],
  },
};
