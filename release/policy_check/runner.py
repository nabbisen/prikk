"""Fixture-table runner for the DC-35 release-policy gate."""

from copy import deepcopy
from pathlib import Path
from typing import Any, Callable
import json

from .challenge import challenge_valid
from .common import DuplicateJsonNameError, pointer_set, strict_json_load, strict_json_loads
from .evidence import evidence_valid, observed_digest, sequence_valid
from .schema import SchemaValidator
from .signer import authority_document_valid, transaction_valid
from .state import release_state_valid


def _load(path: Path) -> dict[str, Any]:
    return strict_json_load(path)


def _load_document(path: Path) -> tuple[dict[str, Any], bytes]:
    raw = path.read_bytes()
    return strict_json_loads(raw), raw


def _fixture_bytes(document: dict[str, Any]) -> bytes:
    return (json.dumps(document, indent=2) + "\n").encode("utf-8")


def _check_rows(path: Path, validator: Callable[[dict[str, Any]], bool]) -> list[str]:
    table = _load(path)
    errors: list[str] = []
    if table.get("schema_version") != 1 or not isinstance(table.get("cases"), list):
        return [f"{path}: invalid fixture table"]
    ids: set[str] = set()
    for case in table["cases"]:
        case_id = case.get("id", "<missing-id>")
        if case_id in ids:
            errors.append(f"{path}:{case_id}: duplicate id")
        ids.add(case_id)
        expected = case.get("expected")
        actual = "valid" if validator(case) else "invalid"
        if actual != expected:
            errors.append(f"{path}:{case_id}: expected {expected}, computed {actual}")
    return errors


def _check_states(path: Path, root: Path, schema_validator: SchemaValidator) -> list[str]:
    def governance_valid(case: dict[str, Any]) -> bool | None:
        reference = case.get("governance")
        if reference is None:
            return None
        if not isinstance(reference, dict):
            return False
        hold_name = reference.get("hold_evidence")
        try:
            hold = _load(root / "fixtures" / f"release-evidence-{hold_name}.json")
        except (FileNotFoundError, TypeError):
            return False
        if schema_validator.validate(hold) or not evidence_valid(hold) or hold["governance"] is None:
            return False
        if hold["governance"]["transaction_type"] != case.get("authority_change"):
            return False
        active = hold["governance"]["hold_ended_at"] is None
        return (case.get("release_hold") == "active") == active

    def validate(case: dict[str, Any]) -> bool:
        valid, local_only = release_state_valid(case, governance_valid(case))
        expected = case["expected"]
        return (expected == "valid" and valid and not local_only) or (
            expected == "valid-local-only" and valid and local_only
        )

    errors: list[str] = []
    table = _load(path)
    ids: set[str] = set()
    for case in table["cases"]:
        case_id = case.get("id", "<missing-id>")
        if case_id in ids:
            errors.append(f"{path}:{case_id}: duplicate id")
        ids.add(case_id)
        accepted = validate(case)
        if case["expected"] == "invalid":
            accepted = not release_state_valid(case, governance_valid(case))[0]
        if not accepted:
            errors.append(f"{path}:{case_id}: computed outcome differs from {case['expected']}")
    return errors


def _check_authority(path: Path) -> list[str]:
    errors: list[str] = []
    for case in _load(path)["cases"]:
        actual = "valid" if authority_document_valid(case["document"]) else "invalid"
        if actual != case["expected"]:
            errors.append(f"{path}:{case['id']}: expected {case['expected']}, computed {actual}")
    return errors


def _mutate(base: dict[str, Any], mutations: list[dict[str, Any]]) -> dict[str, Any]:
    result = deepcopy(base)
    for mutation in mutations:
        pointer_set(result, mutation["path"], mutation["value"])
    return result


def _check_evidence(root: Path, validator: SchemaValidator) -> list[str]:
    fixtures = root / "fixtures"
    base_names = {
        "pending": "release-evidence-pending.json",
        "partial": "release-evidence-partial.json",
        "complete": "release-evidence-complete.json",
        "superseded": "release-evidence-superseded.json",
        "active-hold": "release-evidence-active-hold.json",
        "classified-hold": "release-evidence-classified-hold.json",
        "lifted-hold": "release-evidence-lifted-hold.json",
        "bootstrap-active-hold": "release-evidence-bootstrap-active-hold.json",
        "addition-lifted-hold": "release-evidence-addition-lifted-hold.json",
        "removal-active-hold": "release-evidence-removal-active-hold.json",
    }
    bases = {name: _load_document(fixtures / filename) for name, filename in base_names.items()}
    errors: list[str] = []
    for case in _load(fixtures / "release-evidence-cases.json")["cases"]:
        current_base, base_current_bytes = bases[case["base"]]
        current = _mutate(current_base, case.get("mutations", []))
        if "append_attempt" in case:
            current["attempts"].append(deepcopy(case["append_attempt"]))
        current_bytes = (
            base_current_bytes
            if not case.get("mutations") and "append_attempt" not in case
            else _fixture_bytes(current)
        )
        schema_valid = not validator.validate(current)
        semantic_valid = schema_valid and evidence_valid(current)
        snapshots = [current]
        snapshot_bytes = [current_bytes]
        if "prior_base" in case:
            prior_base, base_prior_bytes = bases[case["prior_base"]]
            prior = _mutate(prior_base, case.get("prior_mutations", []))
            prior_bytes = (
                base_prior_bytes if not case.get("prior_mutations") else _fixture_bytes(prior)
            )
            if case.get("link_prior", False):
                current["prior_snapshot"] = {
                    "name": f"prikk-{prior['version']}-release-evidence-{prior['sequence']}.json",
                    "sha256": observed_digest(prior_bytes),
                }
            if case.get("prior_digest") == "wrong":
                current["prior_snapshot"]["sha256"] = "f" * 64
            current_bytes = _fixture_bytes(current)
            if "current_bytes_base" in case:
                current_bytes = bases[case["current_bytes_base"]][1]
            snapshots = [prior, current]
            snapshot_bytes = [prior_bytes, current_bytes]
            semantic_valid = (
                not validator.validate(prior)
                and not validator.validate(current)
                and sequence_valid(snapshots, snapshot_bytes)
            )
        if schema_valid != case["expected_schema"] or semantic_valid != case["expected_semantic"]:
            errors.append(
                f"{fixtures / 'release-evidence-cases.json'}:{case['id']}: "
                f"schema={schema_valid}, semantic={semantic_valid}"
            )
    return errors


def _check_schema_evaluator(path: Path) -> list[str]:
    errors: list[str] = []
    for case in _load(path)["cases"]:
        try:
            valid = not SchemaValidator(case["schema"]).validate(case["instance"])
            actual = "valid" if valid else "invalid"
        except ValueError:
            actual = "validator-error"
        if actual != case["expected"]:
            errors.append(f"{path}:{case['id']}: expected {case['expected']}, computed {actual}")
    return errors


def _check_json_parser(path: Path, root: Path) -> list[str]:
    errors: list[str] = []
    table = _load(path)
    if table.get("schema_version") != 1 or not isinstance(table.get("cases"), list):
        return [f"{path}: invalid fixture table"]
    ids: set[str] = set()
    for case in table["cases"]:
        case_id = case.get("id", "<missing-id>")
        if case_id in ids:
            errors.append(f"{path}:{case_id}: duplicate id")
        ids.add(case_id)
        try:
            strict_json_load(root / case["path"])
            actual = "valid"
        except DuplicateJsonNameError:
            actual = "duplicate-name-error"
        except (KeyError, OSError, TypeError, UnicodeError, json.JSONDecodeError):
            actual = "parse-error"
        if actual != case.get("expected"):
            errors.append(
                f"{path}:{case_id}: expected {case.get('expected')}, computed {actual}"
            )
    return errors


def run(root: Path) -> int:
    fixtures = root / "fixtures"
    schema = _load(root / "schemas" / "release-evidence-v1.schema.json")
    validator = SchemaValidator(schema)
    errors: list[str] = []
    authority_path = root.parent / "release-signers.toml"
    if not authority_document_valid(authority_path.read_text(encoding="utf-8")):
        errors.append(f"{authority_path}: actual authority file is invalid")
    errors += _check_authority(fixtures / "signer-authority-cases.json")
    errors += _check_rows(fixtures / "signer-governance-cases.json", transaction_valid)
    errors += _check_rows(fixtures / "signer-challenge-cases.json", challenge_valid)
    errors += _check_states(fixtures / "release-state-cases.json", root, validator)
    errors += _check_schema_evaluator(fixtures / "schema-evaluator-cases.json")
    errors += _check_json_parser(fixtures / "json-parser-cases.json", root)
    errors += _check_evidence(root, validator)
    if errors:
        for error in errors:
            print(f"FAIL: {error}")
        return 1
    print("release policy: all fixture outcomes passed")
    return 0
