"""Machine-readable observations from the accepted Python policy validators."""

from copy import deepcopy
from pathlib import Path
from typing import Any, Callable
import json

from .challenge import challenge_valid
from .evidence import evidence_valid, observed_digest, sequence_valid
from .observation_identity import InputIdentity
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


def _observed_record(
    identity: InputIdentity,
    suite_id: str,
    case_id: str,
    case_outcome: str,
    consumed: dict[str, bytes],
    *,
    structural: str | None = None,
    semantic: str | None = None,
) -> dict[str, str]:
    record = _record(
        suite_id,
        case_id,
        case_outcome,
        structural=structural,
        semantic=semantic,
    )
    identity.bind(record, consumed)
    return record


def _boolean_suite(
    path: Path,
    suite_id: str,
    validator: Callable[[dict[str, Any]], bool],
    identity: InputIdentity,
) -> list[dict[str, str]]:
    table_bytes = path.read_bytes()
    return [
        _observed_record(
            identity,
            suite_id,
            case["id"],
            "valid" if validator(case) else "invalid",
            {"fixture-table": table_bytes},
        )
        for case in _load(path)["cases"]
    ]


def _state_observations(
    path: Path,
    root: Path,
    schema: SchemaValidator,
    schema_bytes: bytes,
    identity: InputIdentity,
) -> list[dict[str, str]]:
    def governance(case: dict[str, Any]) -> tuple[bool | None, dict[str, Any] | None]:
        reference = case.get("governance")
        if reference is None:
            return None, None
        if not isinstance(reference, dict):
            return False, None
        name = reference.get("hold_evidence")
        source = root / "fixtures" / f"release-evidence-{name}.json"
        try:
            hold = _load(source)
        except (OSError, TypeError):
            return False, {"state": "absent", "reference": name}
        resolved = {
            "state": "present",
            "reference": name,
            "source_path": source.relative_to(root.parent).as_posix(),
            "document": hold,
        }
        if schema.validate(hold) or not evidence_valid(hold) or hold["governance"] is None:
            return False, resolved
        if hold["governance"]["transaction_type"] != case.get("authority_change"):
            return False, resolved
        valid = (case.get("release_hold") == "active") == (
            hold["governance"]["hold_ended_at"] is None
        )
        return valid, resolved

    records = []
    for case in _load(path)["cases"]:
        governance_valid, resolved = governance(case)
        valid, local_only = release_state_valid(case, governance_valid)
        outcome = "valid-local-only" if valid and local_only else "valid" if valid else "invalid"
        context = _fixture_bytes({"case": case, "governance_evidence": resolved})
        records.append(
            _observed_record(
                identity,
                "release-state",
                case["id"],
                outcome,
                {"fixture-table": context, "schema": schema_bytes},
            )
        )
    return records


def _schema_observations(path: Path, identity: InputIdentity) -> list[dict[str, str]]:
    table_bytes = path.read_bytes()
    records = []
    for case in _load(path)["cases"]:
        try:
            valid = not SchemaValidator(case["schema"]).validate(case["instance"])
            outcome = "valid" if valid else "invalid"
        except ValueError:
            outcome = "validator-error"
        records.append(
            _observed_record(
                identity,
                "schema-evaluator",
                case["id"],
                outcome,
                {"fixture-table": table_bytes},
            )
        )
    return records


def _parser_observations(
    path: Path, root: Path, identity: InputIdentity
) -> list[dict[str, str]]:
    if not path.exists():
        return []
    records = []
    for case in _load(path)["cases"]:
        source = root / case["path"]
        source_bytes = source.read_bytes()
        try:
            _load(source)
            outcome = "valid"
        except json.JSONDecodeError:
            outcome = "parse-error"
        except ValueError as error:
            outcome = (
                "duplicate-name-error"
                if type(error).__name__ == "DuplicateJsonNameError"
                else "parse-error"
            )
        records.append(
            _observed_record(
                identity,
                "json-parser",
                case["id"],
                outcome,
                {"fixture-table": source_bytes},
            )
        )
    return records


def _evidence_observations(
    root: Path,
    validator: SchemaValidator,
    schema_bytes: bytes,
    identity: InputIdentity,
) -> list[dict[str, str]]:
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
        consumed = {
            "schema": schema_bytes,
            "fixture-table": _fixture_bytes({"prior": snapshots[-2] if len(snapshots) > 1 else None, "current": current}),
            "current-snapshot": snapshot_bytes[-1],
        }
        if len(snapshots) > 1:
            consumed["prior-snapshot"] = snapshot_bytes[0]
        records.append(
            _observed_record(
                identity,
                "release-evidence",
                case["id"],
                outcome,
                consumed,
                structural=structural,
                semantic=semantic,
            )
        )
    return records


def observe(root: Path) -> dict[str, Any]:
    fixtures = root / "fixtures"
    identity = InputIdentity(root)
    schema_path = root / "schemas" / "release-evidence-v1.schema.json"
    schema_bytes = schema_path.read_bytes()
    schema = SchemaValidator(_load(schema_path))
    authority_bytes = (root.parent / "release-signers.toml").read_bytes()
    authority_valid = authority_document_valid(
        authority_bytes.decode("utf-8")
    )
    records = [
        _observed_record(
            identity,
            "signer-authority-live",
            "release-signers-toml",
            "valid" if authority_valid else "invalid",
            {"authority": authority_bytes},
        )
    ]
    authority_table = fixtures / "signer-authority-cases.json"
    authority_table_bytes = authority_table.read_bytes()
    records += [
        _observed_record(
            identity,
            "signer-authority",
            case["id"],
            "valid" if authority_document_valid(case["document"]) else "invalid",
            {"fixture-table": authority_table_bytes},
        )
        for case in _load(authority_table)["cases"]
    ]
    records += _boolean_suite(
        fixtures / "signer-governance-cases.json",
        "signer-governance",
        transaction_valid,
        identity,
    )
    for case in _load(fixtures / "signer-challenge-cases.json")["cases"]:
        context = {
            key: value
            for key, value in case.items()
            if key not in {"challenge", "expected"}
        }
        evaluated = {
            **context,
            "challenge": case["challenge"],
            "expected": case["expected"],
        }
        records.append(
            _observed_record(
                identity,
                "signer-challenge",
                case["id"],
                "valid" if challenge_valid(evaluated) else "invalid",
                {
                    "fixture-table": _fixture_bytes(context),
                    "challenge": case["challenge"].encode("utf-8"),
                },
            )
        )
    records += _state_observations(
        fixtures / "release-state-cases.json",
        root,
        schema,
        schema_bytes,
        identity,
    )
    records += _schema_observations(
        fixtures / "schema-evaluator-cases.json", identity
    )
    records += _parser_observations(
        fixtures / "json-parser-cases.json", root, identity
    )
    records += _evidence_observations(root, schema, schema_bytes, identity)
    records.sort(key=lambda item: (item["suite_id"], item["case_id"]))
    return {
        "schema_version": "python-policy-observations-v1",
        "python_baseline_commit": PYTHON_BASELINE_COMMIT,
        "profile_contract_commit": PROFILE_CONTRACT_COMMIT,
        "cases": records,
    }
