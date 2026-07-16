"""Release evidence semantic and cross-snapshot validation."""

from datetime import timedelta
from hashlib import sha256
from typing import Any

from .common import json_equal, parse_datetime, strict_json_loads, unique_sorted_fingerprints

CRATE_ORDER = [
    "prikk-error", "prikk-hash", "prikk-crypto", "prikk-object",
    "prikk-replay", "prikk-store", "prikk",
]
TRANSITIONS = {
    "pending": {"pending", "partial", "complete", "superseded"},
    "partial": {"partial", "complete", "superseded"},
    "complete": {"superseded"},
    "superseded": set(),
}


def observed_digest(snapshot_bytes: bytes) -> str:
    return sha256(snapshot_bytes).hexdigest()


def evidence_valid(snapshot: dict[str, Any]) -> bool:
    version = snapshot["version"]
    if snapshot["tag"]["name"] != version:
        return False
    archive = snapshot["archive"]
    if not _tag_verification_valid(snapshot["tag"]["release_tag_verification"]):
        return False
    if archive["name"] != f"prikk-v{version}.tar.gz":
        return False
    if archive["checksum_name"] != archive["name"] + ".sha256":
        return False
    crates = snapshot["crates"]
    if [crate["name"] for crate in crates] != CRATE_ORDER:
        return False
    if [crate["publish_level"] for crate in crates] != [1, 1, 2, 2, 3, 4, 5]:
        return False
    if any(crate["version"] != version or not crate["exact_internal_requirements"] for crate in crates):
        return False
    if any(not _crate_checksum_state_valid(crate) for crate in crates):
        return False
    governance = snapshot.get("governance")
    if not _governance_valid(governance):
        return False
    if snapshot["overall_status"] == "complete" and _governance_hold_active(governance):
        return False
    try:
        attempt_times = [parse_datetime(item["time"]) for item in snapshot["attempts"]]
    except (TypeError, ValueError):
        return False
    if [item["sequence"] for item in snapshot["attempts"]] != list(range(1, len(snapshot["attempts"]) + 1)):
        return False
    if attempt_times != sorted(attempt_times):
        return False
    return snapshot["overall_status"] != "complete" or _complete_valid(snapshot)


def sequence_valid(snapshots: list[dict[str, Any]], observed_bytes: list[bytes]) -> bool:
    if not snapshots or len(snapshots) != len(observed_bytes):
        return False
    for index, current in enumerate(snapshots):
        try:
            observed = strict_json_loads(observed_bytes[index])
        except (UnicodeDecodeError, ValueError):
            return False
        if not json_equal(current, observed):
            return False
        if current["sequence"] != f"{index + 1:03d}" or not evidence_valid(current):
            return False
        if index == 0:
            if current["prior_snapshot"] is not None:
                return False
            continue
        previous = snapshots[index - 1]
        expected_name = f"prikk-{previous['version']}-release-evidence-{previous['sequence']}.json"
        expected_link = {"name": expected_name, "sha256": observed_digest(observed_bytes[index - 1])}
        if current["prior_snapshot"] != expected_link:
            return False
        if not _immutable_equal(previous, current):
            return False
        if current["overall_status"] not in TRANSITIONS[previous["overall_status"]]:
            return False
        if current["attempts"][: len(previous["attempts"])] != previous["attempts"]:
            return False
        if len(current["attempts"]) <= len(previous["attempts"]):
            return False
    return True


def _immutable_equal(old: dict[str, Any], new: dict[str, Any]) -> bool:
    tag_fields = ("name", "object_id", "peeled_commit")
    archive_fields = ("name", "checksum_name")
    return (
        old["version"] == new["version"]
        and all(old["tag"][field] == new["tag"][field] for field in tag_fields)
        and all(old["archive"][field] == new["archive"][field] for field in archive_fields)
        and _published_values_preserved(old, new)
        and [
            (item["name"], item["version"], item["publish_level"])
            for item in old["crates"]
        ] == [
            (item["name"], item["version"], item["publish_level"])
            for item in new["crates"]
        ]
    )


def _published_values_preserved(old: dict[str, Any], new: dict[str, Any]) -> bool:
    for field in ("archive_sha256", "checksum_sha256"):
        if old["archive"][field] is not None and old["archive"][field] != new["archive"][field]:
            return False
    old_crates = {item["name"]: item for item in old["crates"]}
    for current in new["crates"]:
        previous = old_crates[current["name"]]
        for field in ("staged_sha256", "registry_checksum", "fetched_sha256"):
            if previous[field] is not None and previous[field] != current[field]:
                return False
        if previous["published"] and not current["published"]:
            return False
        if previous["registry_visible"] and not current["registry_visible"]:
            return False
    for field in ("archive_attached", "checksum_attached"):
        if old["archive"][field] and not new["archive"][field]:
            return False
    if old["release_page"]["status"] == "published" and new["release_page"]["status"] != "published":
        return False
    if old["pages"]["deployed_commit"] is not None and old["pages"] != new["pages"]:
        return False
    if not _tag_verification_progression_valid(
        old["tag"]["release_tag_verification"], new["tag"]["release_tag_verification"]
    ):
        return False
    if not _governance_progression_valid(old["governance"], new["governance"]):
        return False
    return True


def _crate_checksum_state_valid(crate: dict[str, Any]) -> bool:
    checksums = [crate["staged_sha256"], crate["registry_checksum"], crate["fetched_sha256"]]
    if crate["checksum_equality"] == "match":
        return None not in checksums and len(set(checksums)) == 1
    if crate["checksum_equality"] == "mismatch":
        return None not in checksums and len(set(checksums)) > 1
    return crate["checksum_equality"] == "not-observed"


def _tag_verification_valid(verification: dict[str, Any]) -> bool:
    details = [
        verification["signer_primary_fingerprint"], verification["authority_path"],
        verification["authority_blob_id"], verification["verifier_result"],
    ]
    if verification["status"] == "not-observed":
        return all(value is None for value in details)
    if verification["status"] == "verified":
        return all(value is not None for value in details)
    if verification["status"] == "failed":
        return (
            verification["authority_path"] is not None
            and verification["authority_blob_id"] is not None
            and verification["verifier_result"] is not None
        )
    return False


def _governance_valid(governance: Any) -> bool:
    if governance is None:
        return True
    old = governance["old_authorized_fingerprints"]
    new = governance["new_authorized_fingerprints"]
    if not unique_sorted_fingerprints(old) or not unique_sorted_fingerprints(new):
        return False
    introduced = sorted(set(new) - set(old))
    removed = sorted(set(old) - set(new))
    transaction = governance["transaction_type"]
    derived = (
        "bootstrap" if not old and introduced and not removed else
        "addition" if introduced and not removed else
        "replacement" if introduced and removed else
        "removal-only" if removed and not introduced else
        "classification-only" if old == new else None
    )
    proof = governance["authority_proof"]
    proofs = proof["introduced_signers"]
    if transaction != derived:
        return False
    old_blob = governance["old_authority_blob_id"]
    new_blob = governance["new_authority_blob_id"]
    if transaction == "classification-only" and old_blob != new_blob:
        return False
    if transaction != "classification-only" and old_blob == new_blob:
        return False
    if introduced:
        if proof["state"] != "verified" or proof["reason"] is not None:
            return False
        if [item["primary_fingerprint"] for item in proofs] != introduced:
            return False
    elif proof["state"] != "not-applicable" or not proof["reason"] or proofs:
        return False
    approvals = governance["approvals"]
    if len(approvals) != 2 or len({item["person"] for item in approvals}) != 2:
        return False
    if {item["role"] for item in approvals} != {"maintainer-administrator", "architect-security"}:
        return False
    try:
        started = parse_datetime(governance["hold_started_at"])
    except (TypeError, ValueError):
        return False
    ended_value = governance["hold_ended_at"]
    lift = governance["hold_lift"]
    classification = governance["classification"]
    if ended_value is None:
        return lift is None
    try:
        ended = parse_datetime(ended_value)
    except (TypeError, ValueError):
        return False
    if lift is None or ended - started < timedelta(hours=72):
        return False
    if transaction == "classification-only" and classification is None:
        return False
    return True


def _governance_hold_active(governance: Any) -> bool:
    return governance is not None and governance["hold_ended_at"] is None


def _tag_verification_progression_valid(old: dict[str, Any], new: dict[str, Any]) -> bool:
    if old["status"] != "not-observed" and old["status"] != new["status"]:
        return False
    for field in ("signer_primary_fingerprint", "authority_path", "authority_blob_id", "verifier_result"):
        if old[field] is not None and old[field] != new[field]:
            return False
    return True


def _governance_progression_valid(old: Any, new: Any) -> bool:
    if old is None:
        return True
    if new is None:
        return False
    fillable = {"classification", "hold_ended_at", "hold_lift"}
    for field, old_value in old.items():
        if field not in fillable and new[field] != old_value:
            return False
        if field in fillable and old_value is not None and new[field] != old_value:
            return False
    return True


def _complete_valid(snapshot: dict[str, Any]) -> bool:
    verification = snapshot["tag"]["release_tag_verification"]
    if verification["status"] != "verified" or any(
        verification[field] is None
        for field in ("signer_primary_fingerprint", "authority_path", "authority_blob_id", "verifier_result")
    ):
        return False
    archive = snapshot["archive"]
    if not all((archive["archive_attached"], archive["checksum_attached"])):
        return False
    if archive["checksum_grammar"] != "valid" or archive["archive_root"] != "valid":
        return False
    for crate in snapshot["crates"]:
        checksums = [crate["staged_sha256"], crate["registry_checksum"], crate["fetched_sha256"]]
        if None in checksums or len(set(checksums)) != 1 or crate["checksum_equality"] != "match":
            return False
        if not crate["published"] or not crate["registry_visible"]:
            return False
    pages = snapshot["pages"]
    if pages["status"] == "deployed" and pages["deployed_commit"] != snapshot["tag"]["peeled_commit"]:
        return False
    if pages["status"] == "inapplicable" and not pages["inapplicable_ruling"]:
        return False
    return snapshot["release_page"]["status"] == "published" and pages["status"] in {"deployed", "inapplicable"}
