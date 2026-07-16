"""Signer authority and governance transaction checks."""

from typing import Any
import tomllib

from .common import unique_sorted_fingerprints

TYPES = {"bootstrap", "addition", "replacement", "removal-only", "classification-only"}
ROLES = {"maintainer-administrator", "architect-security"}


def authority_document_valid(document: str) -> bool:
    try:
        parsed = tomllib.loads(document)
    except (tomllib.TOMLDecodeError, TypeError):
        return False
    return (
        set(parsed) == {"schema_version", "authorized_primary_fingerprints"}
        and type(parsed["schema_version"]) is int
        and parsed["schema_version"] == 1
        and unique_sorted_fingerprints(parsed["authorized_primary_fingerprints"])
    )


def transaction_valid(case: dict[str, Any]) -> bool:
    required = {"id", "old", "new", "declared_type", "proof", "approvals", "expected"}
    if set(case) != required:
        return False
    old, new = case["old"], case["new"]
    if not unique_sorted_fingerprints(old) or not unique_sorted_fingerprints(new):
        return False
    introduced = sorted(set(new) - set(old))
    removed = sorted(set(old) - set(new))
    actual_type = _transaction_type(old, new, introduced, removed)
    if actual_type is None or case["declared_type"] not in TYPES or case["declared_type"] != actual_type:
        return False
    if not _approvals_valid(case["approvals"]):
        return False
    return _proof_valid(case["proof"], introduced)


def _transaction_type(old: list[str], new: list[str], introduced: list[str], removed: list[str]) -> str | None:
    if not old and introduced and not removed:
        return "bootstrap"
    if introduced and not removed:
        return "addition"
    if introduced and removed:
        return "replacement"
    if removed and not introduced:
        return "removal-only"
    if old == new:
        return "classification-only"
    return None


def _approvals_valid(approvals: Any) -> bool:
    if not isinstance(approvals, list) or len(approvals) != 2:
        return False
    if any(not isinstance(item, dict) or set(item) != {"person", "role"} for item in approvals):
        return False
    people = [item["person"] for item in approvals]
    roles = [item["role"] for item in approvals]
    return (
        all(isinstance(person, str) and person and not person.startswith("automation-") for person in people)
        and len(set(people)) == 2
        and set(roles) == ROLES
    )


def _proof_valid(proof: Any, introduced: list[str]) -> bool:
    if not isinstance(proof, dict) or set(proof) != {"state", "reason", "introduced_signers"}:
        return False
    records = proof["introduced_signers"]
    if not isinstance(records, list):
        return False
    if introduced:
        if proof["state"] != "verified" or proof["reason"] is not None:
            return False
        fingerprints = [record.get("primary_fingerprint") for record in records if isinstance(record, dict)]
        return (
            all(
                set(record) == {"primary_fingerprint", "verifier_result"}
                and isinstance(record["verifier_result"], str)
                and bool(record["verifier_result"])
                for record in records
                if isinstance(record, dict)
            )
            and len(records) == len(fingerprints)
            and fingerprints == introduced
        )
    return (
        proof["state"] == "not-applicable"
        and isinstance(proof["reason"], str)
        and bool(proof["reason"])
        and records == []
    )
