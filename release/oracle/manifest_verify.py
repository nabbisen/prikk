"""Narrow structural and exact-byte verification for oracle-manifest-v1."""

from collections import Counter
from pathlib import Path
from typing import Any

from coverage_contract import (
    expected_inventory, input_bytes, load_packs, location_key, oracle_id,
    verify_file_identity, verify_pack_closure,
)

REASON_ORDER = [
    "manifest-contract", "input-identity", "json-syntax-or-duplicate-name",
    "schema-profile-or-compilation", "schema-instance", "authority-grammar",
    "challenge-grammar-or-binding", "challenge-time-window",
    "governance-transition-or-proof", "governance-review-or-hold", "release-state",
    "evidence-byte-identity-or-link", "evidence-transition-or-attempt-prefix",
    "evidence-tag-or-artifact", "evidence-completion", "none",
]
def _snapshot_name(snapshot: dict[str, Any]) -> str:
    return f"prikk-{snapshot['version']}-release-evidence-{snapshot['sequence']}.json"


def _expected_relations(case: dict[str, Any], errors: list[str]) -> None:
    expected = case["expected"]
    suite = case["suite_id"]
    outcome = expected["case_outcome"]
    final = expected["final"]
    stages = [expected["structural"], expected["semantic"]]
    if stages[0] in {"invalid", "validator-error"} and stages[1] != "not-run":
        errors.append(f"manifest-contract:stage-not-run:{suite}:{case['case_id']}")
    executed = [stage for stage in stages if stage != "not-run"]
    derived = (
        "validator-error"
        if "validator-error" in executed
        else "invalid"
        if "invalid" in executed
        else "valid"
        if executed
        else final
    )
    if derived != final:
        errors.append(f"manifest-contract:stage-final:{suite}:{case['case_id']}")
    outcome_final = (
        "valid"
        if outcome in {"valid", "valid-local-only"}
        else "validator-error"
        if outcome == "validator-error"
        else "invalid"
    )
    if outcome_final != final:
        errors.append(f"manifest-contract:outcome-final:{suite}:{case['case_id']}")
    if outcome == "valid-local-only" and (suite != "release-state" or final != "valid"):
        errors.append(f"manifest-contract:valid-local-only:{suite}:{case['case_id']}")
    if outcome in {"duplicate-name-error", "parse-error"} and (
        suite != "json-parser" or final != "invalid"
    ):
        errors.append(f"manifest-contract:parser-outcome:{suite}:{case['case_id']}")
    if final == "valid" and expected["primary_reason"] != "none":
        errors.append(f"manifest-contract:valid-reason:{suite}:{case['case_id']}")
    if final != "valid" and expected["primary_reason"] == "none":
        errors.append(f"manifest-contract:invalid-reason:{suite}:{case['case_id']}")
    if expected["primary_reason"] not in REASON_ORDER:
        errors.append(f"manifest-contract:reason:{suite}:{case['case_id']}")


def _verify_state_context(
    root: Path, case: dict[str, Any], content: bytes | None, errors: list[str]
) -> None:
    from policy_check.common import strict_json_load, strict_json_loads

    key = f"{case['suite_id']}:{case['case_id']}"
    if content is None:
        return
    try:
        context = strict_json_loads(content)
    except ValueError:
        errors.append(f"manifest-contract:state-context-json:{key}")
        return
    if set(context) != {"case", "governance_evidence"}:
        errors.append(f"manifest-contract:state-context-fields:{key}")
        return
    state_case = context["case"]
    if not isinstance(state_case, dict) or state_case.get("id") != case["fixture_case_id"]:
        errors.append(f"manifest-contract:state-context-case:{key}")
        return
    reference = state_case.get("governance")
    evidence = context["governance_evidence"]
    if reference is None:
        if evidence is not None:
            errors.append(f"manifest-contract:state-context-unexpected-governance:{key}")
        return
    if not isinstance(reference, dict) or set(reference) != {"hold_evidence"}:
        errors.append(f"manifest-contract:state-context-reference:{key}")
        return
    name = reference["hold_evidence"]
    expected_path = f"release/fixtures/release-evidence-{name}.json"
    path = root / expected_path
    if path.exists():
        expected = {
            "state": "present", "reference": name, "source_path": expected_path,
            "document": strict_json_load(path),
        }
    else:
        expected = {"state": "absent", "reference": name}
    if evidence != expected:
        errors.append(f"manifest-contract:state-context-governance:{key}")


def _verify_cases(
    root: Path,
    manifest: dict[str, Any],
    payloads: dict[tuple[str, str], bytes],
    errors: list[str],
) -> set[str]:
    cases = manifest["cases"]
    keys = [(case["suite_id"], case["case_id"]) for case in cases]
    if keys != sorted(keys) or len(keys) != len(set(keys)):
        errors.append("manifest-contract:case-order-or-duplicate")
    references: Counter[tuple[str, str]] = Counter()
    for case in cases:
        key = f"{case['suite_id']}:{case['case_id']}"
        if case["case_id"] != oracle_id(case["fixture_case_id"]):
            errors.append(f"manifest-contract:fixture-case-binding:{key}")
        inputs = case["inputs"]
        roles = [item["role"] for item in inputs]
        expected_roles = {
            "signer-authority-live": ["authority", "expected-output"],
            "signer-challenge": ["fixture-table", "challenge", "expected-output"],
            "release-state": ["fixture-table", "schema", "expected-output"],
            "json-parser": ["fixture-table", "expected-output"],
            "release-evidence": ["schema", "fixture-table", "current-snapshot", "expected-output"],
        }.get(case["suite_id"], ["fixture-table", "expected-output"])
        if case["suite_id"] == "release-evidence" and "sequence" in case:
            expected_roles.insert(2, "prior-snapshot")
        if roles != expected_roles:
            errors.append(f"manifest-contract:input-roles:{key}")
        if [item["ordinal"] for item in inputs] != list(range(len(inputs))):
            errors.append(f"manifest-contract:input-order:{key}")
        locations = [location_key(item) for item in inputs]
        if len(locations) != len(set(locations)):
            errors.append(f"manifest-contract:input-location-duplicate:{key}")
        output = next((item for item in inputs if item["role"] == "expected-output"), None)
        output_location = None if output is None else output["location"]
        if output_location != {
            "kind": "direct", "path": "release/oracle/python-observations-v1.json"
        }:
            errors.append(f"manifest-contract:expected-output-role:{key}")
        resolved = []
        packed_roles = {
            "signer-challenge": {"fixture-table", "challenge"},
            "release-state": {"fixture-table"},
            "release-evidence": {"fixture-table", "prior-snapshot", "current-snapshot"},
        }
        for item in inputs:
            location = item["location"]
            if location["kind"] == "packed":
                reference = (location["pack_id"], location["entry_id"])
                references[reference] += 1
                if location["pack_id"] != case["suite_id"] or item["role"] not in packed_roles.get(
                    case["suite_id"], set()
                ):
                    errors.append(f"manifest-contract:packed-suite-role:{key}:{item['role']}")
            resolved.append(input_bytes(root, item, payloads, errors))
        if case["suite_id"] == "release-state":
            state_index = next(
                index for index, item in enumerate(inputs) if item["role"] == "fixture-table"
            )
            _verify_state_context(root, case, resolved[state_index], errors)
        _expected_relations(case, errors)
        sequence = case.get("sequence")
        if sequence is not None:
            if len(sequence) != 2:
                errors.append(f"manifest-contract:sequence-size:{key}")
                continue
            snapshot_inputs = [
                (index, item) for index, item in enumerate(inputs)
                if item["role"] in {"prior-snapshot", "current-snapshot"}
            ]
            expected_roles = ["prior-snapshot", "current-snapshot"]
            if [item["role"] for _, item in snapshot_inputs] != expected_roles:
                errors.append(f"manifest-contract:sequence-input-roles:{key}")
                continue
            from policy_check.common import strict_json_loads
            if any(resolved[index] is None for index, _ in snapshot_inputs):
                errors.append(f"manifest-contract:sequence-input-missing:{key}")
                continue
            try:
                snapshots = [strict_json_loads(resolved[index]) for index, _ in snapshot_inputs]
            except ValueError:
                errors.append(f"manifest-contract:sequence-json:{key}")
                continue
            expected_names = [_snapshot_name(snapshot) for snapshot in snapshots]
            for index, member in enumerate(sequence):
                ordinal = member["input_ordinal"]
                if not isinstance(ordinal, int) or not 0 <= ordinal < len(inputs):
                    errors.append(f"manifest-contract:sequence-ordinal:{key}:{index}")
                    continue
                source = inputs[ordinal]
                expected_ordinal, expected_input = snapshot_inputs[index]
                if ordinal != expected_ordinal or source is not expected_input:
                    errors.append(f"manifest-contract:sequence-role:{key}:{index}")
                for name in ("byte_length", "sha256"):
                    if member[name] != source[name]:
                        errors.append(f"manifest-contract:sequence-{name}:{key}:{index}")
                if index == 0 and member["predecessor_name"] is not None:
                    errors.append(f"manifest-contract:sequence-first-predecessor:{key}")
                if index > 0 and member["predecessor_name"] != sequence[index - 1]["current_name"]:
                    errors.append(f"manifest-contract:sequence-link:{key}:{index}")
                expected_predecessor = None if index == 0 else expected_names[index - 1]
                if member["predecessor_name"] != expected_predecessor:
                    errors.append(f"manifest-contract:sequence-predecessor-name:{key}:{index}")
                if member["current_name"] != expected_names[index]:
                    errors.append(f"manifest-contract:sequence-current-name:{key}:{index}")
    verify_pack_closure(payloads, references, errors)
    return {f"{suite}:{case}" for suite, case in keys}


def _verify_observations(root: Path, manifest: dict[str, Any], errors: list[str]) -> None:
    from policy_check.common import strict_json_load

    output = strict_json_load(root / "release" / "oracle" / "python-observations-v1.json")
    expected_identity = {
        "schema_version": "python-policy-observations-v1",
        "python_baseline_commit": "12c137d",
        "profile_contract_commit": "ea427df",
    }
    for name, value in expected_identity.items():
        if output.get(name) != value:
            errors.append(f"manifest-contract:observation-{name}")
    output_cases = {(item["suite_id"], item["case_id"]): item for item in output.get("cases", [])}
    manifest_cases = {
        (item["suite_id"], item["fixture_case_id"]): item for item in manifest["cases"]
    }
    if output_cases.keys() != manifest_cases.keys():
        errors.append("manifest-contract:observation-case-set")
        return
    for key, case in manifest_cases.items():
        observed = output_cases[key]
        expected = case["expected"]
        for name in ("final", "case_outcome"):
            if observed.get(name) != expected[name]:
                errors.append(f"manifest-contract:observation-{name}:{key[0]}:{key[1]}")
        for name in ("structural", "semantic"):
            if name in observed and observed[name] != expected[name]:
                errors.append(f"manifest-contract:observation-{name}:{key[0]}:{key[1]}")


def verify_coverage(
    root: Path,
    manifest: dict[str, Any],
    payloads: dict[tuple[str, str], bytes],
    errors: list[str],
    coverage: dict[str, Any] | None = None,
) -> None:
    from policy_check.common import strict_json_load

    if coverage is None:
        coverage = strict_json_load(root / "release" / "oracle" / "coverage-inventory-v1.json")
    try:
        expected = expected_inventory(root, manifest, payloads)
    except (KeyError, TypeError, ValueError) as error:
        errors.append(f"manifest-contract:coverage-derivation:{error}")
        return
    if coverage != expected:
        errors.append("manifest-contract:coverage-exact")


def _verify_reason_map(root: Path, manifest: dict[str, Any], errors: list[str]) -> None:
    from policy_check.common import strict_json_load

    verify_file_identity(root, manifest["reason_map"], errors)
    if manifest["reason_map"]["path"] != "release/oracle/reason-map-v1.json":
        errors.append("manifest-contract:reason-map-path")
        return
    reason_map = strict_json_load(root / manifest["reason_map"]["path"])
    expected = {
        f"{case['suite_id']}:{case['fixture_case_id']}": case["expected"]["primary_reason"]
        for case in manifest["cases"] if case["expected"]["final"] != "valid"
    }
    if reason_map != expected:
        errors.append("manifest-contract:reason-map-exact")


def verify(root: Path, manifest: dict[str, Any], schema: dict[str, Any]) -> list[str]:
    from policy_check.schema import SchemaValidator

    errors = SchemaValidator(schema).validate(manifest)
    errors = [f"manifest-contract:schema:{error}" for error in errors]
    if errors:
        return errors
    verify_file_identity(root, manifest["normative_schema"], errors)
    if manifest["normative_schema"]["path"] != "release/schemas/release-evidence-v1.schema.json":
        errors.append("manifest-contract:normative-schema-path")
    payloads = load_packs(root, manifest, schema, errors)
    _verify_cases(root, manifest, payloads, errors)
    _verify_reason_map(root, manifest, errors)
    _verify_observations(root, manifest, errors)
    verify_coverage(root, manifest, payloads, errors)
    return errors
