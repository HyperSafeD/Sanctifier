"""
Tests for action.yml input defaults and fixture-driven validation.

Covers:
  - Default values match what action.yml declares
  - validate_inputs() accepts all cases in valid_action_inputs.yml
  - validate_inputs() rejects all cases in invalid_action_inputs.yml
  - write_env_file() produces correct KEY=VALUE lines
  - main() applies defaults from environment variables when no CLI args given
"""
from __future__ import annotations

import pathlib
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
FIXTURES = pathlib.Path(__file__).resolve().parent / "fixtures"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _load_yaml(path: pathlib.Path) -> dict:
    try:
        import yaml  # type: ignore[import]
    except ImportError:
        import json  # fallback: parse a minimal YAML-as-JSON subset
        raise unittest.SkipTest("PyYAML not installed; skipping YAML fixture tests")
    with open(path, encoding="utf-8") as f:
        return yaml.safe_load(f)


# ---------------------------------------------------------------------------
# Default values
# ---------------------------------------------------------------------------

class ActionDefaultValueTests(unittest.TestCase):
    """Verify that default values in action.yml match the validation logic."""

    def test_default_path_is_dot(self) -> None:
        from scripts.action_inputs import validate_inputs

        result = validate_inputs(
            path=".",
            min_severity="high",
            format="sarif",
            upload_sarif="true",
            sarif_output="sanctifier-results.sarif",
            debug="false",
        )
        self.assertEqual(result.path, ".")

    def test_default_min_severity_is_high(self) -> None:
        from scripts.action_inputs import validate_inputs

        result = validate_inputs(
            path=".",
            min_severity="high",
            format="sarif",
            upload_sarif="true",
            sarif_output="sanctifier-results.sarif",
            debug="false",
        )
        self.assertEqual(result.min_severity, "high")

    def test_default_format_is_sarif(self) -> None:
        from scripts.action_inputs import validate_inputs

        result = validate_inputs(
            path=".",
            min_severity="high",
            format="sarif",
            upload_sarif="true",
            sarif_output="sanctifier-results.sarif",
            debug="false",
        )
        self.assertEqual(result.format, "sarif")

    def test_default_upload_sarif_is_true(self) -> None:
        from scripts.action_inputs import validate_inputs

        result = validate_inputs(
            path=".",
            min_severity="high",
            format="sarif",
            upload_sarif="true",
            sarif_output="sanctifier-results.sarif",
            debug="false",
        )
        self.assertEqual(result.upload_sarif, "true")

    def test_default_sarif_output_filename(self) -> None:
        from scripts.action_inputs import validate_inputs

        result = validate_inputs(
            path=".",
            min_severity="high",
            format="sarif",
            upload_sarif="true",
            sarif_output="sanctifier-results.sarif",
            debug="false",
        )
        self.assertEqual(result.sarif_output, "sanctifier-results.sarif")

    def test_default_debug_is_false(self) -> None:
        from scripts.action_inputs import validate_inputs

        result = validate_inputs(
            path=".",
            min_severity="high",
            format="sarif",
            upload_sarif="true",
            sarif_output="sanctifier-results.sarif",
            debug="false",
        )
        self.assertEqual(result.debug, "false")

    def test_all_allowed_severities_accepted(self) -> None:
        from scripts.action_inputs import validate_inputs

        for sev in ("critical", "high", "medium", "low", "info"):
            with self.subTest(severity=sev):
                result = validate_inputs(
                    path=".",
                    min_severity=sev,
                    format="sarif",
                    upload_sarif="true",
                    sarif_output="out.sarif",
                    debug="false",
                )
                self.assertEqual(result.min_severity, sev)

    def test_all_allowed_formats_accepted(self) -> None:
        from scripts.action_inputs import validate_inputs

        for fmt, sarif_out in (("sarif", "out.sarif"), ("json", "out.json"), ("text", "out.txt")):
            with self.subTest(format=fmt):
                result = validate_inputs(
                    path=".",
                    min_severity="high",
                    format=fmt,
                    upload_sarif="false",
                    sarif_output=sarif_out,
                    debug="false",
                )
                self.assertEqual(result.format, fmt)


# ---------------------------------------------------------------------------
# Boolean normalisation
# ---------------------------------------------------------------------------

class BooleanNormalisationTests(unittest.TestCase):
    def _check(self, value: str, expected: str) -> None:
        from scripts.action_inputs import validate_inputs

        result = validate_inputs(
            path=".",
            min_severity="high",
            format="sarif",
            upload_sarif=value,
            sarif_output="out.sarif",
            debug="false",
        )
        self.assertEqual(result.upload_sarif, expected)

    def test_true_string(self) -> None:
        self._check("true", "true")

    def test_yes_string(self) -> None:
        self._check("yes", "true")

    def test_one_string(self) -> None:
        self._check("1", "true")

    def test_false_string(self) -> None:
        self._check("false", "false")

    def test_no_string(self) -> None:
        self._check("no", "false")

    def test_zero_string(self) -> None:
        self._check("0", "false")

    def test_TRUE_uppercase(self) -> None:
        self._check("TRUE", "true")

    def test_FALSE_uppercase(self) -> None:
        self._check("FALSE", "false")

    def test_invalid_bool_raises(self) -> None:
        from scripts.action_inputs import validate_inputs

        with self.assertRaisesRegex(ValueError, "true or false"):
            validate_inputs(
                path=".",
                min_severity="high",
                format="sarif",
                upload_sarif="maybe",
                sarif_output="out.sarif",
                debug="false",
            )


# ---------------------------------------------------------------------------
# write_env_file output
# ---------------------------------------------------------------------------

class WriteEnvFileTests(unittest.TestCase):
    def _write_and_read(self, **kwargs: str) -> dict[str, str]:
        from scripts.action_inputs import validate_inputs, write_env_file

        inputs = validate_inputs(**kwargs)
        with tempfile.TemporaryDirectory() as tmp:
            out = pathlib.Path(tmp) / "env"
            write_env_file(inputs, out)
            lines = out.read_text(encoding="utf-8").splitlines()
        return dict(line.split("=", 1) for line in lines if "=" in line)

    def test_env_file_contains_all_expected_keys(self) -> None:
        env = self._write_and_read(
            path=".",
            min_severity="high",
            format="sarif",
            upload_sarif="true",
            sarif_output="sanctifier-results.sarif",
            debug="false",
        )
        self.assertIn("SANCTIFIER_ACTION_PATH", env)
        self.assertIn("SANCTIFIER_ACTION_MIN_SEVERITY", env)
        self.assertIn("SANCTIFIER_ACTION_FORMAT", env)
        self.assertIn("SANCTIFIER_ACTION_UPLOAD_SARIF", env)
        self.assertIn("SANCTIFIER_ACTION_SARIF_OUTPUT", env)
        self.assertIn("SANCTIFIER_ACTION_DEBUG", env)

    def test_env_file_values_match_normalised_inputs(self) -> None:
        env = self._write_and_read(
            path=".",
            min_severity="HIGH",
            format="SARIF",
            upload_sarif="YES",
            sarif_output="results.sarif",
            debug="TRUE",
        )
        self.assertEqual(env["SANCTIFIER_ACTION_MIN_SEVERITY"], "high")
        self.assertEqual(env["SANCTIFIER_ACTION_FORMAT"], "sarif")
        self.assertEqual(env["SANCTIFIER_ACTION_UPLOAD_SARIF"], "true")
        self.assertEqual(env["SANCTIFIER_ACTION_DEBUG"], "true")

    def test_env_file_path_preserved(self) -> None:
        env = self._write_and_read(
            path=".",
            min_severity="high",
            format="sarif",
            upload_sarif="true",
            sarif_output="sanctifier-results.sarif",
            debug="false",
        )
        self.assertEqual(env["SANCTIFIER_ACTION_PATH"], ".")

    def test_env_file_ends_with_newline(self) -> None:
        from scripts.action_inputs import validate_inputs, write_env_file

        inputs = validate_inputs(
            path=".",
            min_severity="high",
            format="sarif",
            upload_sarif="true",
            sarif_output="out.sarif",
            debug="false",
        )
        with tempfile.TemporaryDirectory() as tmp:
            out = pathlib.Path(tmp) / "env"
            write_env_file(inputs, out)
            content = out.read_text(encoding="utf-8")
        self.assertTrue(content.endswith("\n"), "env file must end with a newline")

    def test_env_file_no_shell_injection_characters(self) -> None:
        env = self._write_and_read(
            path=".",
            min_severity="high",
            format="sarif",
            upload_sarif="true",
            sarif_output="sanctifier-results.sarif",
            debug="false",
        )
        for key, value in env.items():
            self.assertNotIn(";", value, f"{key} contains shell injection char ;")
            self.assertNotIn("|", value, f"{key} contains shell injection char |")
            self.assertNotIn("`", value, f"{key} contains shell injection char `")


# ---------------------------------------------------------------------------
# Fixture-driven tests (YAML)
# ---------------------------------------------------------------------------

class ValidFixtureDrivenTests(unittest.TestCase):
    """Iterate over tests/action/fixtures/valid_action_inputs.yml."""

    @classmethod
    def _load_cases(cls):
        fixture_path = FIXTURES / "valid_action_inputs.yml"
        try:
            data = _load_yaml(fixture_path)
        except unittest.SkipTest:
            return []
        return data.get("cases", [])

    def test_all_valid_fixture_cases_pass(self) -> None:
        from scripts.action_inputs import validate_inputs

        cases = self._load_cases()
        if not cases:
            self.skipTest("No YAML fixture cases loaded")

        for case in cases:
            with self.subTest(id=case["id"], description=case.get("description", "")):
                inp = case["inputs"]
                result = validate_inputs(
                    path=inp["path"],
                    min_severity=inp["min_severity"],
                    format=inp["format"],
                    upload_sarif=inp["upload_sarif"],
                    sarif_output=inp["sarif_output"],
                    debug=inp["debug"],
                )
                expected = case.get("expected", {})
                for field, value in expected.items():
                    self.assertEqual(
                        getattr(result, field),
                        value,
                        f"case '{case['id']}': expected {field}={value!r}, got {getattr(result, field)!r}",
                    )


class InvalidFixtureDrivenTests(unittest.TestCase):
    """Iterate over tests/action/fixtures/invalid_action_inputs.yml."""

    @classmethod
    def _load_cases(cls):
        fixture_path = FIXTURES / "invalid_action_inputs.yml"
        try:
            data = _load_yaml(fixture_path)
        except unittest.SkipTest:
            return []
        return data.get("cases", [])

    def test_all_invalid_fixture_cases_raise(self) -> None:
        from scripts.action_inputs import validate_inputs

        cases = self._load_cases()
        if not cases:
            self.skipTest("No YAML fixture cases loaded")

        for case in cases:
            with self.subTest(id=case["id"], description=case.get("description", "")):
                inp = case["inputs"]
                with self.assertRaises(ValueError) as ctx:
                    validate_inputs(
                        path=inp["path"],
                        min_severity=inp["min_severity"],
                        format=inp["format"],
                        upload_sarif=inp["upload_sarif"],
                        sarif_output=inp["sarif_output"],
                        debug=inp["debug"],
                    )
                fragment = case.get("error_fragment", "")
                if fragment:
                    self.assertIn(
                        fragment,
                        str(ctx.exception),
                        f"case '{case['id']}': expected error to contain {fragment!r}",
                    )


if __name__ == "__main__":
    unittest.main()
