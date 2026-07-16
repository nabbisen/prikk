"""Release lifecycle state-row validation."""

from typing import Any


def release_state_valid(case: dict[str, Any], governance_valid: bool | None = None) -> tuple[bool, bool]:
    known = {
        "id", "workspace", "latest", "candidate", "changelog", "rfc", "tag", "distribution",
        "internal_requirements", "missing_output", "authority_change", "release_hold", "dispute",
        "governance", "expected",
    }
    if set(case) - known:
        return False, False
    state = _classify(case)
    if state is None:
        return False, False
    if case.get("internal_requirements", "exact") != "exact":
        return False, False
    if case.get("missing_output") is not None and case["distribution"] == "complete":
        return False, False
    authority_change = case.get("authority_change", "absent")
    release_hold = case.get("release_hold", "lifted")
    dispute = case.get("dispute", "none")
    if state == "development":
        transaction_types = {"bootstrap", "addition", "replacement", "removal-only", "classification-only"}
        if authority_change != "absent" and (
            authority_change not in transaction_types
            or governance_valid is not True
            or release_hold not in {"active", "lifted"}
        ):
            return False, False
        if authority_change == "absent" and (
            governance_valid is not None or release_hold != "lifted" or dispute != "none"
        ):
            return False, False
        if dispute not in {"none", "active"}:
            return False, False
    elif authority_change != "absent" or release_hold != "lifted" or dispute != "none":
        return False, False
    local_only = state == "private-finalization"
    return True, local_only


def _classify(case: dict[str, Any]) -> str | None:
    fields = (
        case.get("workspace"), case.get("latest"), case.get("candidate"), case.get("changelog"),
        case.get("rfc"), case.get("tag"), case.get("distribution"),
    )
    valid = {
        (
            "last-release", "last-release", None, "no-target-claim",
            "proposed-or-accepted", "absent-at-head", "pending",
        ): "development",
        ("target", "last-release", "target", "candidate", "accepted", "absent", "pending"): "release-candidate",
        ("target", "target", None, "final", "done", "local-only-matching", "pending"): "private-finalization",
        ("target", "target", None, "final", "done", "public-matching", "pending"): "released",
        ("target", "target", None, "final", "done", "public-matching", "partial"): "released",
        ("target", "target", None, "final", "done", "public-matching", "complete"): "released",
    }
    return valid.get(fields)
