"""Independent expectations and negative assurance for policy observations."""

from copy import deepcopy
from pathlib import Path
from typing import Any

from .observation import observe
from .runner import _load


def _expected_record(
    suite_id: str,
    case_id: str,
    case_outcome: str,
    *,
    structural: str | None = None,
    semantic: str | None = None,
) -> dict[str, str]:
    if case_outcome in {"valid", "valid-local-only"}:
        final = "valid"
    elif case_outcome == "validator-error":
        final = "validator-error"
    else:
        final = "invalid"
    record = {
        "suite_id": suite_id,
        "case_id": case_id,
        "final": final,
        "case_outcome": case_outcome,
    }
    if structural is not None:
        record["structural"] = structural
    if semantic is not None:
        record["semantic"] = semantic
    return record


def _expected_simple(path: Path, suite_id: str) -> list[dict[str, str]]:
    return [
        _expected_record(suite_id, case["id"], case["expected"])
        for case in _load(path)["cases"]
    ]


def _expected_observations(root: Path) -> list[dict[str, str]]:
    fixtures = root / "fixtures"
    records = [_expected_record("signer-authority-live", "release-signers-toml", "valid")]
    records += _expected_simple(fixtures / "signer-authority-cases.json", "signer-authority")
    records += _expected_simple(fixtures / "signer-governance-cases.json", "signer-governance")
    records += _expected_simple(fixtures / "signer-challenge-cases.json", "signer-challenge")
    records += _expected_simple(fixtures / "release-state-cases.json", "release-state")
    records += _expected_simple(fixtures / "schema-evaluator-cases.json", "schema-evaluator")
    parser_path = fixtures / "json-parser-cases.json"
    if parser_path.exists():
        records += _expected_simple(parser_path, "json-parser")
    for case in _load(fixtures / "release-evidence-cases.json")["cases"]:
        structural_valid = case["expected_schema"]
        semantic_valid = case["expected_semantic"]
        records.append(
            _expected_record(
                "release-evidence",
                case["id"],
                "valid" if structural_valid and semantic_valid else "invalid",
                structural="valid" if structural_valid else "invalid",
                semantic=(
                    "valid"
                    if semantic_valid
                    else "invalid"
                    if structural_valid
                    else "not-run"
                ),
            )
        )
    records.sort(key=lambda item: (item["suite_id"], item["case_id"]))
    return records


def _expected_document(root: Path) -> dict[str, Any]:
    return {
        "schema_version": "python-policy-observations-v1",
        "python_baseline_commit": "12c137d",
        "profile_contract_commit": "ea427df",
        "cases": _expected_observations(root),
    }


def verify_observation_document(root: Path, actual: dict[str, Any]) -> list[str]:
    """Compare a supplied document with fixture-owned expected outcomes."""

    expected = _expected_document(root)
    if actual == expected:
        return []
    errors = []
    expected_fields = set(expected)
    if set(actual) != expected_fields:
        errors.append(
            f"document-contract: expected fields {sorted(expected_fields)}, "
            f"computed {sorted(actual)}"
        )
    for name in ("schema_version", "python_baseline_commit", "profile_contract_commit"):
        if actual.get(name) != expected[name]:
            errors.append(f"{name}: expected {expected[name]}, computed {actual.get(name)}")
    actual_cases = actual.get("cases")
    if not isinstance(actual_cases, list):
        errors.append("cases: expected an ordered list")
        return errors
    expected_cases = expected["cases"]
    actual_by_key = {(item["suite_id"], item["case_id"]): item for item in actual_cases}
    expected_by_key = {(item["suite_id"], item["case_id"]): item for item in expected_cases}
    for key in sorted(actual_by_key.keys() | expected_by_key.keys()):
        if actual_by_key.get(key) != expected_by_key.get(key):
            errors.append(
                f"{key[0]}:{key[1]}: expected {expected_by_key.get(key)}, "
                f"computed {actual_by_key.get(key)}"
            )
    actual_keys = [(item["suite_id"], item["case_id"]) for item in actual_cases]
    expected_keys = [(item["suite_id"], item["case_id"]) for item in expected_cases]
    if actual_keys != expected_keys:
        errors.append("case-order: records are not in required suite/case order")
    return errors


def verify_observations(root: Path) -> list[str]:
    return verify_observation_document(root, observe(root))


def self_test(root: Path) -> list[str]:
    """Prove final-only and identity drift are rejected by the verifier."""

    document = observe(root)
    wrong_final = deepcopy(document)
    target = next(item for item in wrong_final["cases"] if item["final"] == "invalid")
    target["final"] = "valid"
    target_prefix = f"{target['suite_id']}:{target['case_id']}:"
    if not any(
        error.startswith(target_prefix)
        for error in verify_observation_document(root, wrong_final)
    ):
        return ["self-test: final-only observation drift was not rejected"]

    wrong_identity = deepcopy(document)
    wrong_identity["schema_version"] = "incorrect"
    wrong_identity["python_baseline_commit"] = "incorrect"
    wrong_identity["profile_contract_commit"] = "incorrect"
    identity_errors = verify_observation_document(root, wrong_identity)
    required = {"schema_version", "python_baseline_commit", "profile_contract_commit"}
    observed = {error.split(":", 1)[0] for error in identity_errors}
    if not required <= observed:
        return ["self-test: top-level identity drift was not rejected"]
    return []
