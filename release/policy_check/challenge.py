"""Canonical signer proof challenge parser."""

from datetime import timedelta
from typing import Any
import re

from .common import FINGERPRINT, GIT_ID, parse_datetime

FIELD_NAMES = [
    "repository",
    "primary_fingerprint",
    "role",
    "authority_revision",
    "nonce",
    "issued_at",
    "expires_at",
]


def challenge_valid(case: dict[str, Any]) -> bool:
    if set(case) != {
        "id", "challenge", "expected_authority_revision", "observed_at",
        "observed_primary_fingerprint", "verifier_result", "expected",
    }:
        return False
    challenge = case["challenge"]
    if not isinstance(challenge, str) or not challenge.isascii() or not challenge.endswith("\n"):
        return False
    if "\r" in challenge or challenge.endswith("\n\n"):
        return False
    lines = challenge[:-1].split("\n")
    if len(lines) != 8 or lines[0] != "prikk-release-signer-proof-v1":
        return False
    fields: dict[str, str] = {}
    for expected_name, line in zip(FIELD_NAMES, lines[1:], strict=True):
        if not line.startswith(expected_name + "="):
            return False
        fields[expected_name] = line[len(expected_name) + 1 :]
    if fields["repository"] != "https://github.com/nabbisen/prikk":
        return False
    if not FINGERPRINT.fullmatch(fields["primary_fingerprint"]):
        return False
    if fields["role"] != "official-release" or not GIT_ID.fullmatch(fields["authority_revision"]):
        return False
    if not re.fullmatch(r"[0-9a-f]{64}", fields["nonce"]):
        return False
    try:
        issued = parse_datetime(fields["issued_at"])
        expires = parse_datetime(fields["expires_at"])
        observed = parse_datetime(case["observed_at"])
    except (TypeError, ValueError):
        return False
    if not issued < expires or expires - issued > timedelta(hours=24):
        return False
    if issued - observed > timedelta(minutes=5) or observed >= expires:
        return False
    return (
        case["expected_authority_revision"] == fields["authority_revision"]
        and case["observed_primary_fingerprint"] == fields["primary_fingerprint"]
        and isinstance(case["verifier_result"], str)
        and bool(case["verifier_result"])
    )
