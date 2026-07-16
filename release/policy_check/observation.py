"""Machine-readable observations from the accepted Python policy validators."""

from copy import deepcopy
from pathlib import Path
from typing import Any, Callable
import json

from .challenge import challenge_valid
from .evidence import evidence_valid, observed_digest, sequence_valid
from .runner import _fixture_bytes, _load, _load_document, _mutate
from .schema import SchemaValidator
from .signer import authority_document_valid, transaction_valid
from .state import release_state_valid

PYTHON_BASELINE_COMMIT = "12c137d"
PROFILE_CONTRACT_COMMIT = "ea427df"


def _record(
    suite_id: str,
    case_id: str,
    case_outcome: str,
    *,
    structural: str | None = None,
    semantic: str | None = None,
) -> dict[str, str]:
    final = {
        "valid": "valid",
        "valid-local-only": "valid",
        "validator-error": "validator-error",
    }.get(case_outcome, "invalid")
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


def _boolean_suite(
    path: Path, suite_id: str, validator: Callable[[dict[str, Any]], bool]
) -> list[dict[str, str]]:
    return [
        _record(suite_id, case["id"], "valid" if validator(case) else "invalid")
        for case in _load(path)["cases"]
    ]


def _state_observations(path: Path, root: Path, schema: SchemaValidator) -> list[dict[str, str]]:
    def governance_valid(case: dict[str, Any]) -> bool | None:
        reference = case.get("governance")
        if reference is None:
            return None
        if not isinstance(reference, dict):
            return False
        try:
            hold = _load(root / "fixtures" / f"release-evidence-{reference.get('hold_evidence')}.json")
        except (OSError, TypeError):
            return False
        if schema.validate(hold) or not evidence_valid(hold) or hold["governance"] is None:
            return False
        if hold["governance"]["transaction_type"] != case.get("authority_change"):
            return False
        return (case.get("release_hold") == "active") == (
            hold["governance"]["hold_ended_at"] is None
        )

    records = []
    for case in _load(path)["cases"]:
        valid, local_only = release_state_valid(case, governance_valid(case))
        outcome = "valid-local-only" if valid and local_only else "valid" if valid else "invalid"
        records.append(_record("release-state", case["id"], outcome))
    return records


def _schema_observations(path: Path) -> list[dict[str, str]]:
    records = []
    for case in _load(path)["cases"]:
        try:
            valid = not SchemaValidator(case["schema"]).validate(case["instance"])
            outcome = "valid" if valid else "invalid"
        except ValueError:
            outcome = "validator-error"
        records.append(_record("schema-evaluator", case["id"], outcome))
    return records


def _parser_observations(path: Path, root: Path) -> list[dict[str, str]]:
    if not path.exists():
        return []
    records = []
    for case in _load(path)["cases"]:
        try:
            _load(root / case["path"])
            outcome = "valid"
        except json.JSONDecodeError:
            outcome = "parse-error"
        except ValueError as error:
            outcome = (
                "duplicate-name-error"
                if type(error).__name__ == "DuplicateJsonNameError"
                else "parse-error"
            )
        records.append(_record("json-parser", case["id"], outcome))
    return records


def _evidence_observations(root: Path, validator: SchemaValidator) -> list[dict[str, str]]:
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
    bases = {name: _load_document(fixtures / file) for name, file in base_names.items()}
    records = []
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
        snapshots = [current]
        snapshot_bytes = [current_bytes]
        if "prior_base" in case:
            prior_base, base_prior_bytes = bases[case["prior_base"]]
            prior = _mutate(prior_base, case.get("prior_mutations", []))
            prior_bytes = base_prior_bytes if not case.get("prior_mutations") else _fixture_bytes(prior)
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

        structural_valid = all(not validator.validate(snapshot) for snapshot in snapshots)
        semantic_valid = structural_valid and (
            sequence_valid(snapshots, snapshot_bytes)
            if len(snapshots) > 1
            else evidence_valid(current)
        )
        structural = "valid" if structural_valid else "invalid"
        semantic = "valid" if semantic_valid else "invalid" if structural_valid else "not-run"
        outcome = "valid" if structural_valid and semantic_valid else "invalid"
        records.append(
            _record(
                "release-evidence",
                case["id"],
                outcome,
                structural=structural,
                semantic=semantic,
            )
        )
    return records


def observe(root: Path) -> dict[str, Any]:
    fixtures = root / "fixtures"
    schema = SchemaValidator(_load(root / "schemas" / "release-evidence-v1.schema.json"))
    authority_valid = authority_document_valid(
        (root.parent / "release-signers.toml").read_text(encoding="utf-8")
    )
    records = [_record("signer-authority-live", "release-signers-toml", "valid" if authority_valid else "invalid")]
    records += [
        _record(
            "signer-authority",
            case["id"],
            "valid" if authority_document_valid(case["document"]) else "invalid",
        )
        for case in _load(fixtures / "signer-authority-cases.json")["cases"]
    ]
    records += _boolean_suite(fixtures / "signer-governance-cases.json", "signer-governance", transaction_valid)
    records += _boolean_suite(fixtures / "signer-challenge-cases.json", "signer-challenge", challenge_valid)
    records += _state_observations(fixtures / "release-state-cases.json", root, schema)
    records += _schema_observations(fixtures / "schema-evaluator-cases.json")
    records += _parser_observations(fixtures / "json-parser-cases.json", root)
    records += _evidence_observations(root, schema)
    records.sort(key=lambda item: (item["suite_id"], item["case_id"]))
    return {
        "schema_version": "python-policy-observations-v1",
        "python_baseline_commit": PYTHON_BASELINE_COMMIT,
        "profile_contract_commit": PROFILE_CONTRACT_COMMIT,
        "cases": records,
    }
