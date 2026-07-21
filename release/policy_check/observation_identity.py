"""Fail-closed identities for the exact inputs consumed by policy observations."""

from hashlib import sha256
from pathlib import Path
from typing import Any

from .runner import _load


class InputIdentity:
    """Verify consumed bytes against one frozen case before emitting its digest."""

    def __init__(self, release_root: Path) -> None:
        self._release_root = release_root
        manifest = _load(release_root / "oracle" / "oracle-manifest-v1.json")
        self._cases = {
            (case["suite_id"], case["fixture_case_id"]): case
            for case in manifest["cases"]
        }

    def bind(
        self,
        record: dict[str, str],
        consumed: dict[str, bytes],
    ) -> None:
        key = (record["suite_id"], record["case_id"])
        case = self._cases.get(key)
        if case is None:
            raise ValueError(f"observation input identity case absent: {key[0]}:{key[1]}")
        evaluation_roles = {
            item["role"] for item in case["inputs"] if item["role"] != "expected-output"
        }
        if set(consumed) != evaluation_roles:
            raise ValueError(f"observation consumed role mismatch: {key[0]}:{key[1]}")
        record["input_digest"] = self._digest(case, consumed)

    def bind_expected(self, records: list[dict[str, str]]) -> None:
        keys = {(record["suite_id"], record["case_id"]) for record in records}
        if keys != self._cases.keys():
            raise ValueError("expected observation case set differs from oracle manifest")
        for record in records:
            case = self._cases[(record["suite_id"], record["case_id"])]
            consumed = {
                item["role"]: self._resolve(item)
                for item in case["inputs"]
                if item["role"] != "expected-output"
            }
            record["input_digest"] = self._digest(case, consumed)

    def _digest(self, case: dict[str, Any], consumed: dict[str, bytes]) -> str:
        digest = sha256()
        for item in sorted(case["inputs"], key=lambda value: value["ordinal"]):
            role = item["role"]
            content = self._resolve(item) if role == "expected-output" else consumed[role]
            actual_sha256 = sha256(content).hexdigest()
            if len(content) != item["byte_length"] or actual_sha256 != item["sha256"]:
                raise ValueError(
                    f"observation consumed input mismatch: "
                    f"{case['suite_id']}:{case['fixture_case_id']}:{role}"
                )
            location = item["location"]
            if location["kind"] == "direct":
                location_text = f"direct:{location['path']}"
            else:
                location_text = (
                    f"packed:{location['pack_id']}:{location['entry_id']}"
                )
            binding = (
                f"ordinal={item['ordinal']}\n"
                f"role={role}\n"
                f"location={location_text}\n"
                f"byte_length={len(content)}\n"
                f"sha256={actual_sha256}\n"
            )
            digest.update(binding.encode("utf-8"))
        return digest.hexdigest()

    def _resolve(self, item: dict[str, Any]) -> bytes:
        location = item["location"]
        if location["kind"] == "direct":
            return (self._release_root.parent / location["path"]).read_bytes()
        pack_path = self._release_root / "oracle" / "packs" / (
            f"{location['pack_id']}-v1.json"
        )
        pack = _load(pack_path)
        entry = next(
            (
                value
                for value in pack["entries"]
                if value["entry_id"] == location["entry_id"]
            ),
            None,
        )
        if entry is None:
            raise ValueError(f"observation packed input absent: {location['entry_id']}")
        return entry["content"].encode("utf-8")
