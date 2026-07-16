"""Shared strict parsing and comparison helpers."""

from datetime import datetime
import re
from typing import Any

FINGERPRINT = re.compile(r"(?:[0-9A-F]{40}|[0-9A-F]{64})\Z")
GIT_ID = re.compile(r"(?:[0-9a-f]{40}|[0-9a-f]{64})\Z")
SHA256 = re.compile(r"[0-9a-f]{64}\Z")


def parse_datetime(value: str) -> datetime:
    if not isinstance(value, str) or not re.fullmatch(
        r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z", value
    ):
        raise ValueError("date-time must use canonical UTC second precision")
    return datetime.fromisoformat(value.replace("Z", "+00:00"))


def unique_sorted_fingerprints(values: Any) -> bool:
    return (
        isinstance(values, list)
        and all(isinstance(value, str) and FINGERPRINT.fullmatch(value) for value in values)
        and values == sorted(set(values))
    )


def json_equal(left: Any, right: Any) -> bool:
    if isinstance(left, bool) or isinstance(right, bool):
        return isinstance(left, bool) and isinstance(right, bool) and left == right
    if isinstance(left, (int, float)) and isinstance(right, (int, float)):
        return left == right
    if type(left) is not type(right):
        return False
    if isinstance(left, list):
        return len(left) == len(right) and all(json_equal(a, b) for a, b in zip(left, right, strict=True))
    if isinstance(left, dict):
        return left.keys() == right.keys() and all(json_equal(left[key], right[key]) for key in left)
    return left == right


def pointer_set(document: Any, pointer: str, value: Any) -> None:
    parts = [part.replace("~1", "/").replace("~0", "~") for part in pointer.split("/")[1:]]
    target = document
    for part in parts[:-1]:
        target = target[int(part)] if isinstance(target, list) else target[part]
    last = parts[-1]
    if value == {"$delete": True}:
        if isinstance(target, list):
            del target[int(last)]
        else:
            del target[last]
    elif isinstance(target, list):
        target[int(last)] = value
    else:
        target[last] = value
