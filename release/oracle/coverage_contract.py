"""Exact reviewed coverage contract for oracle-manifest-v1."""

from collections import Counter
from hashlib import sha256
from pathlib import Path, PurePosixPath
from typing import Any
import json
import re
import shutil

from policy_check.common import strict_json_loads
from policy_check.schema import SchemaValidator

SUBJECTS = (
    "authority", "challenge", "release-state", "schema", "governance", "transition",
    "exact-byte", "tag", "hold", "completion", "sequence",
)
STATUSES = ("pending", "partial", "complete", "superseded")
PACK_PATHS = {
    "release-evidence": "release/oracle/packs/release-evidence-v1.json",
    "release-state": "release/oracle/packs/release-state-v1.json",
    "signer-challenge": "release/oracle/packs/signer-challenge-v1.json",
}
PATH_SEGMENT = re.compile(r"[A-Za-z0-9_.-]+\Z")
REPAIR_REGRESSIONS = {
    "schema-evaluator": {
        "boolean_and_integer_are_unique", "boolean_enum_distinct_from_integer",
        "integer_and_equivalent_number_are_duplicates", "integer_const_rejects_boolean",
        "unknown_assertion_keyword_fails_closed",
    },
    "json-parser": {
        "duplicate_evidence_field", "duplicate_fixture_case_field", "duplicate_name_in_array_object",
        "duplicate_nested_name", "duplicate_schema_keyword", "duplicate_top_level_name",
        "escaped_equivalent_duplicate_name", "malformed_json", "unique_object_names",
    },
    "release-evidence": {
        "raw_predecessor_digest_mismatch", "full_schema_boolean_version_rejected",
        "governance_active_to_classified", "governance_classified_to_lifted",
        "governance_premature_lift", "governance_post_lift_classification_mutation",
        "tag_verification_signer_primary_fingerprint_immutable",
        "tag_verification_authority_path_immutable", "tag_verification_authority_blob_id_immutable",
        "tag_verification_verifier_result_immutable", "tag_verification_status_immutable",
        "sequence_zero_attempt_growth", "snapshot_object_bytes_mismatch",
        "pending_verified_without_details", "partial_verified_without_details",
        "pending_not_observed_with_detail", "pending_failed_without_authority",
        "partial_failed_without_authority", "pending_failed_with_authority_and_result",
        "canonical_bootstrap_hold_transaction", "canonical_addition_lifted_transaction",
        "canonical_removal_hold_transaction", "governance_fingerprint_set_proof_mismatch",
        "governance_approval_identity_mismatch", "governance_authority_blob_transition_mismatch",
        "governance_declared_type_mismatch",
    },
}


def oracle_id(fixture_id: str) -> str:
    return fixture_id.replace("_", "-")

def case_key(case: dict[str, Any]) -> str:
    return f"{case['suite_id']}:{case['case_id']}"


def _subject_membership(case: dict[str, Any]) -> set[str]:
    suite = case["suite_id"]
    fixture_id = case["fixture_case_id"]
    subjects = set()
    if suite.startswith("signer-authority"):
        subjects.add("authority")
    if suite == "signer-challenge":
        subjects.update(("challenge", "exact-byte"))
    if suite == "release-state":
        subjects.add("release-state")
    if suite in {"schema-evaluator", "json-parser"} or case["expected"]["structural"] == "invalid":
        subjects.add("schema")
    if suite == "signer-governance" or "governance" in fixture_id:
        subjects.add("governance")
    if fixture_id.startswith("transition_"):
        subjects.add("transition")
    if any(word in fixture_id for word in (
        "raw_", "digest", "object_bytes", "golden_bytes", "crlf", "final_lf",
    )):
        subjects.add("exact-byte")
    if "tag" in fixture_id:
        subjects.add("tag")
    if "hold" in fixture_id or "governance" in fixture_id:
        subjects.add("hold")
    if any(word in fixture_id for word in (
        "complete", "checksum", "archive", "crate", "pages", "release_page",
    )):
        subjects.add("completion")
    if "sequence" in case:
        subjects.add("sequence")
    return subjects
def _transition(
    root: Path, case: dict[str, Any], payloads: dict[tuple[str, str], bytes]
) -> dict[str, Any]:
    def content(item: dict[str, Any]) -> bytes:
        location = item["location"]
        if location["kind"] == "direct":
            return (root / location["path"]).read_bytes()
        return payloads[(location["pack_id"], location["entry_id"])]

    snapshots = {
        item["role"]: strict_json_loads(content(item))
        for item in case["inputs"]
        if item["role"] in {"prior-snapshot", "current-snapshot"}
    }
    source = snapshots["prior-snapshot"]["overall_status"]
    target = snapshots["current-snapshot"]["overall_status"]
    expected_fixture_id = f"transition_{source}_to_{target}"
    if case["fixture_case_id"] != expected_fixture_id:
        raise ValueError(f"transition-case-name:{case_key(case)}")
    return {
        "case_key": case_key(case),
        "from": source,
        "to": target,
        "expected_valid": case["expected"]["final"] == "valid",
    }
def expected_inventory(
    root: Path,
    manifest: dict[str, Any],
    payloads: dict[tuple[str, str], bytes],
) -> dict[str, Any]:
    cases = manifest["cases"]
    suites: dict[str, int] = {}
    reasons: dict[str, int] = {}
    subjects = {name: [] for name in SUBJECTS}
    transitions = []
    for case in cases:
        suite = case["suite_id"]
        suites[suite] = suites.get(suite, 0) + 1
        reason = case["expected"]["primary_reason"]
        reasons[reason] = reasons.get(reason, 0) + 1
        for subject in _subject_membership(case):
            subjects[subject].append(case_key(case))
        if suite == "release-evidence" and case["fixture_case_id"].startswith("transition_"):
            transitions.append(_transition(root, case, payloads))
    regressions = sorted(
        f"{suite}:{oracle_id(fixture_id)}"
        for suite, fixture_ids in REPAIR_REGRESSIONS.items()
        for fixture_id in fixture_ids
    )
    keys = {case_key(case) for case in cases}
    if any(not members for members in subjects.values()):
        raise ValueError("empty-subject")
    if not set(regressions).issubset(keys):
        raise ValueError("unknown-repair-regression")
    pairs = {(item["from"], item["to"]) for item in transitions}
    required_pairs = {(source, target) for source in STATUSES for target in STATUSES}
    if len(transitions) != len(required_pairs) or pairs != required_pairs:
        raise ValueError("transition-pair-set")
    return {
        "schema_version": "oracle-coverage-v1",
        "total_cases": len(cases),
        "suites": [
            {"suite_id": name, "case_count": count} for name, count in sorted(suites.items())
        ],
        "reason_counts": [
            {"primary_reason": name, "case_count": count}
            for name, count in sorted(reasons.items())
        ],
        "subjects": [
            {"subject": name, "case_keys": sorted(subjects[name])} for name in SUBJECTS
        ],
        "transition_pairs": sorted(transitions, key=lambda item: (item["from"], item["to"])),
        "repair_regressions": regressions,
    }
def lexical_path(value: Any) -> bool:
    if not isinstance(value, str) or "\\" in value:
        return False
    parts = value.split("/")
    return bool(parts) and all(
        part not in {"", ".", ".."} and PATH_SEGMENT.fullmatch(part) for part in parts
    )


def repository_file(root: Path, value: Any, errors: list[str]) -> Path | None:
    if not lexical_path(value):
        errors.append(f"manifest-contract:path:{value!r}")
        return None
    path = PurePosixPath(value)
    candidate = root.joinpath(*path.parts)
    try:
        resolved = candidate.resolve(strict=True)
    except OSError:
        errors.append(f"input-identity:missing:{value}")
        return None
    if not resolved.is_relative_to(root.resolve()):
        errors.append(f"input-identity:outside-root:{value}")
        return None
    if not resolved.is_file():
        errors.append(f"input-identity:not-regular:{value}")
        return None
    return candidate


def verify_file_identity(root: Path, item: dict[str, Any], errors: list[str]) -> bytes | None:
    path = repository_file(root, item.get("path"), errors)
    if path is None:
        return None
    content = path.read_bytes()
    if item.get("byte_length") != len(content):
        errors.append(f"input-identity:length:{item.get('path')}")
    if item.get("sha256") != sha256(content).hexdigest():
        errors.append(f"input-identity:sha256:{item.get('path')}")
    return content


def _contains_surrogate(value: Any) -> bool:
    if isinstance(value, str):
        return any(0xD800 <= ord(character) <= 0xDFFF for character in value)
    if isinstance(value, list):
        return any(_contains_surrogate(item) for item in value)
    if isinstance(value, dict):
        return any(_contains_surrogate(key) or _contains_surrogate(item) for key, item in value.items())
    return False


def parse_pack(
    content: bytes, pack_id: str, schema: dict[str, Any], errors: list[str]
) -> dict[str, bytes]:
    if content.startswith(b"\xef\xbb\xbf"):
        errors.append(f"manifest-contract:pack-bom:{pack_id}")
        return {}
    try:
        pack = strict_json_loads(content.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError, ValueError) as error:
        errors.append(f"manifest-contract:pack-json:{pack_id}:{type(error).__name__}")
        return {}
    if _contains_surrogate(pack):
        errors.append(f"manifest-contract:pack-surrogate:{pack_id}")
        return {}
    pack_schema = {"$ref": "#/$defs/vectorPack", "$defs": schema["$defs"]}
    schema_errors = SchemaValidator(pack_schema).validate(pack)
    if schema_errors:
        errors.extend(f"manifest-contract:pack-schema:{pack_id}:{error}" for error in schema_errors)
        return {}
    entries = pack["entries"]
    entry_ids = [entry["entry_id"] for entry in entries]
    if entry_ids != sorted(entry_ids) or len(entry_ids) != len(set(entry_ids)):
        errors.append(f"manifest-contract:pack-entry-order:{pack_id}")
        return {}
    if any(not lexical_path(entry_id) for entry_id in entry_ids):
        errors.append(f"manifest-contract:pack-entry-path:{pack_id}")
        return {}
    prefix = f"release/oracle/vectors/{pack_id}/"
    if any(not entry_id.startswith(prefix) for entry_id in entry_ids):
        errors.append(f"manifest-contract:pack-suite:{pack_id}")
    return {entry["entry_id"]: entry["content"].encode("utf-8") for entry in entries}


def load_packs(
    root: Path, manifest: dict[str, Any], schema: dict[str, Any], errors: list[str]
) -> dict[tuple[str, str], bytes]:
    pack_root = root / "release" / "oracle" / "packs"
    observed_files = {
        path.relative_to(root).as_posix() for path in pack_root.rglob("*") if path.is_file()
    }
    if observed_files != set(PACK_PATHS.values()):
        errors.append("manifest-contract:pack-file-set")
    registry = manifest["packs"]
    if [(item["pack_id"], item["path"]) for item in registry] != sorted(PACK_PATHS.items()):
        errors.append("manifest-contract:pack-registry")
    payloads = {}
    for item in registry:
        pack_id = item["pack_id"]
        content = verify_file_identity(root, item, errors)
        if content is None or PACK_PATHS.get(pack_id) != item["path"]:
            continue
        for entry_id, payload in parse_pack(content, pack_id, schema, errors).items():
            payloads[(pack_id, entry_id)] = payload
    return payloads


def location_key(item: dict[str, Any]) -> tuple[str, ...]:
    location = item["location"]
    if location["kind"] == "direct":
        return ("direct", location["path"])
    return ("packed", location["pack_id"], location["entry_id"])


def input_bytes(
    root: Path, item: dict[str, Any], payloads: dict[tuple[str, str], bytes], errors: list[str]
) -> bytes | None:
    location = item["location"]
    if location["kind"] == "direct":
        path = repository_file(root, location["path"], errors)
        content = None if path is None else path.read_bytes()
        label = location["path"]
    else:
        key = (location["pack_id"], location["entry_id"])
        content = payloads.get(key) if lexical_path(location["entry_id"]) else None
        label = f"{key[0]}:{key[1]}"
        if content is None:
            category = "pack-entry-missing" if lexical_path(location["entry_id"]) else "path"
            errors.append(f"manifest-contract:{category}:{label}")
    if content is None:
        return None
    if item["byte_length"] != len(content):
        errors.append(f"input-identity:length:{label}")
    if item["sha256"] != sha256(content).hexdigest():
        errors.append(f"input-identity:sha256:{label}")
    return content


def verify_pack_closure(
    payloads: dict[tuple[str, str], bytes], references: Counter[tuple[str, str]], errors: list[str]
) -> None:
    if references != Counter({key: 1 for key in payloads}):
        errors.append("manifest-contract:pack-entry-closure")


def copy_candidate(root: Path, destination: Path) -> None:
    shutil.copy2(root / "release-signers.toml", destination / "release-signers.toml")
    for name in ("fixtures", "schemas", "oracle"):
        shutil.copytree(root / "release" / name, destination / "release" / name)


def closure_self_test(payloads: dict[tuple[str, str], bytes]) -> bool:
    exact = Counter({key: 1 for key in payloads})
    removed = next(iter(exact))
    omitted = exact.copy()
    del omitted[removed]
    duplicate = exact.copy()
    duplicate[removed] = 2
    extras = dict(payloads)
    extras[("release-state", "release/oracle/vectors/release-state/extra/context.json")] = b"x"
    substituted = omitted.copy()
    substituted[next(iter(substituted))] += 1
    candidates = ((payloads, omitted), (payloads, duplicate), (extras, exact), (payloads, substituted))
    for candidate_payloads, references in candidates:
        errors: list[str] = []
        verify_pack_closure(candidate_payloads, references, errors)
        if "manifest-contract:pack-entry-closure" not in errors:
            return False
    return True
