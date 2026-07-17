"""Negative assurance for oracle-manifest-v1 verification."""

from copy import deepcopy
from hashlib import sha256
from pathlib import Path
from typing import Any
import json
import tempfile
from manifest_verify import verify, verify_coverage
from coverage_contract import closure_self_test, copy_candidate, load_packs, parse_pack
from policy_check.common import strict_json_load

def _case(manifest: dict[str, Any], case_id: str) -> dict[str, Any]:
    return next(case for case in manifest["cases"] if case["case_id"] == case_id)


def _pack(entries: list[dict[str, str]], **extra: object) -> bytes:
    value = {"schema_version": "oracle-vector-pack-v1", "entries": entries, **extra}
    return json.dumps(value, ensure_ascii=False).encode()


def _entry(name: str = "a", content: str = "value") -> dict[str, str]:
    return {
        "entry_id": f"release/oracle/vectors/signer-challenge/{name}/challenge.txt",
        "content": content,
    }


def _pack_profile_self_test(schema: dict[str, Any]) -> list[str]:
    invalid = {
        "malformed": b"{",
        "duplicate-name": (
            b'{"schema_version":"oracle-vector-pack-v1","entries":[],"entries":[]}'
        ),
        "nested-duplicate-name": (
            b'{"schema_version":"oracle-vector-pack-v1","entries":[{"entry_id":"release/'
            b'oracle/vectors/signer-challenge/a/challenge.txt","content":"a","content":"b"}]}'
        ),
        "bom": b"\xef\xbb\xbf" + _pack([_entry()]),
        "unknown-field": _pack([_entry()], unexpected=True),
        "missing-field": b'{"schema_version":"oracle-vector-pack-v1"}',
        "unsupported-version": _pack([_entry()]).replace(b"oracle-vector-pack-v1", b"wrong"),
        "duplicate-entry": _pack([_entry(), _entry()]),
        "unsorted-entry": _pack([_entry("b"), _entry("a")]),
        "unknown-entry-field": _pack([{**_entry(), "unexpected": "x"}]),
        "missing-entry-field": _pack([{"entry_id": _entry()["entry_id"]}]),
        "lone-high-surrogate": (
            b'{"schema_version":"oracle-vector-pack-v1","entries":[{"entry_id":"release/'
            b'oracle/vectors/signer-challenge/a/challenge.txt","content":"\\ud800"}]}'
        ),
        "lone-low-surrogate": (
            b'{"schema_version":"oracle-vector-pack-v1","entries":[{"entry_id":"release/'
            b'oracle/vectors/signer-challenge/a/challenge.txt","content":"\\udc00"}]}'
        ),
        "wrong-suite-entry": _pack([{
            "entry_id": "release/oracle/vectors/release-state/a/context.json", "content": "x",
        }]),
        "dot-entry": _pack([{
            "entry_id": "release/oracle/vectors/signer-challenge/./a/challenge.txt", "content": "x",
        }]),
        "dot-dot-entry": _pack([{
            "entry_id": "release/oracle/vectors/signer-challenge/../a/challenge.txt", "content": "x",
        }]),
    }
    for name, content in invalid.items():
        errors: list[str] = []
        parse_pack(content, "signer-challenge", schema, errors)
        if not errors:
            return [f"self-test:pack-{name}-not-rejected"]

    pair = (
        b'{"schema_version":"oracle-vector-pack-v1","entries":[{"entry_id":"release/'
        b'oracle/vectors/signer-challenge/a/challenge.txt","content":"\\ud83d\\ude00"}]}'
    )
    errors = []
    pair_payload = parse_pack(pair, "signer-challenge", schema, errors)
    if errors or next(iter(pair_payload.values()), None) != "😀".encode():
        return ["self-test:pack-surrogate-pair-not-preserved"]

    values = ["é", "a\r\nb", "\r", "\n", "no-final-lf", "e\u0301", "\u0000\t"]
    for index, value in enumerate(values):
        raw = _pack([_entry(content=value)])
        escaped = json.dumps(
            {"schema_version": "oracle-vector-pack-v1", "entries": [_entry(content=value)]},
            ensure_ascii=True,
        ).encode()
        raw_errors: list[str] = []
        escaped_errors: list[str] = []
        raw_value = parse_pack(raw, "signer-challenge", schema, raw_errors)
        escaped_value = parse_pack(escaped, "signer-challenge", schema, escaped_errors)
        expected = value.encode()
        if raw_errors or escaped_errors or list(raw_value.values()) != [expected] or (
            list(escaped_value.values()) != [expected]
        ):
            return [f"self-test:pack-scalar-preservation:{index}"]
    if values[0].encode() == values[5].encode():
        return ["self-test:pack-normalization-distinction-lost"]
    return []


def _filesystem_self_test(
    root: Path, manifest: dict[str, Any], schema: dict[str, Any]
) -> list[str]:
    with tempfile.TemporaryDirectory(prefix="prikk-oracle-") as directory:
        candidate = Path(directory)
        copy_candidate(root, candidate)
        extra = candidate / "release" / "oracle" / "packs" / "extra.json"
        extra.write_text("{}\n", encoding="utf-8")
        if "manifest-contract:pack-file-set" not in verify(candidate, manifest, schema):
            return ["self-test:physical-extra-pack-not-rejected"]
        extra.unlink()

        direct_alias = deepcopy(manifest)
        direct = next(
            item for case in direct_alias["cases"] for item in case["inputs"]
            if item["location"]["kind"] == "direct" and "/" in item["location"]["path"]
        )
        direct["location"]["path"] = direct["location"]["path"].replace("/", "/./", 1)
        if not any(error.startswith("manifest-contract:") for error in verify(candidate, direct_alias, schema)):
            return ["self-test:direct-dot-alias-not-rejected"]

        registry_alias = deepcopy(manifest)
        registry_alias["packs"][0]["path"] = registry_alias["packs"][0]["path"].replace("/", "/./", 1)
        if not any(error.startswith("manifest-contract:") for error in verify(candidate, registry_alias, schema)):
            return ["self-test:registry-dot-alias-not-rejected"]

        pack_path = candidate / "release" / "oracle" / "packs" / "signer-challenge-v1.json"
        original = pack_path.read_bytes()
        for segment in ("./", "../"):
            changed = deepcopy(manifest)
            packed = next(
                item for case in changed["cases"] for item in case["inputs"]
                if item["location"]["kind"] == "packed"
                and item["location"]["pack_id"] == "signer-challenge"
            )
            old_id = packed["location"]["entry_id"]
            new_id = old_id.replace("signer-challenge/", f"signer-challenge/{segment}", 1)
            pack = json.loads(original)
            next(entry for entry in pack["entries"] if entry["entry_id"] == old_id)["entry_id"] = new_id
            content = (json.dumps(pack, indent=2, ensure_ascii=False) + "\n").encode()
            pack_path.write_bytes(content)
            packed["location"]["entry_id"] = new_id
            registry = next(item for item in changed["packs"] if item["pack_id"] == "signer-challenge")
            registry.update(byte_length=len(content), sha256=sha256(content).hexdigest())
            if not any(error.startswith("manifest-contract:") for error in verify(candidate, changed, schema)):
                return [f"self-test:packed-{segment}-alias-not-rejected"]
            pack_path.write_bytes(original)
    return []


def _location_and_closure_self_test(
    root: Path,
    manifest: dict[str, Any],
    schema: dict[str, Any],
    payloads: dict[tuple[str, str], bytes],
) -> list[str]:
    direct_case = next(
        case for case in manifest["cases"]
        if any(item["location"]["kind"] == "direct" for item in case["inputs"])
    )
    packed_case = next(
        case for case in manifest["cases"]
        if any(item["location"]["kind"] == "packed" for item in case["inputs"])
    )
    direct_index = next(
        index for index, item in enumerate(direct_case["inputs"])
        if item["location"]["kind"] == "direct"
    )
    packed_index = next(
        index for index, item in enumerate(packed_case["inputs"])
        if item["location"]["kind"] == "packed"
    )

    location_mutations = {
        "both": {"kind": "direct", "path": "release-signers.toml", "pack_id": "release-state", "entry_id": "x"},
        "neither": {"kind": "direct"},
        "mixed": {"kind": "packed", "pack_id": "release-state", "entry_id": "x", "path": "x"},
        "wrong-kind": {"kind": "unknown", "path": "release-signers.toml"},
        "traversal": {"kind": "direct", "path": "../outside"},
    }
    for name, location in location_mutations.items():
        changed = deepcopy(manifest)
        _case(changed, direct_case["case_id"])["inputs"][direct_index]["location"] = location
        if not verify(root, changed, schema):
            return [f"self-test:location-{name}-not-rejected"]

    malformed_id = deepcopy(manifest)
    item = _case(malformed_id, packed_case["case_id"])["inputs"][packed_index]
    item["location"]["entry_id"] = "bad//entry"
    if not verify(root, malformed_id, schema):
        return ["self-test:packed-id-not-rejected"]

    absent_pack = deepcopy(manifest)
    absent_pack["packs"][0]["path"] = "release/oracle/packs/absent.json"
    if not verify(root, absent_pack, schema):
        return ["self-test:absent-pack-not-rejected"]
    omitted_pack = deepcopy(manifest)
    omitted_pack["packs"].pop()
    if not verify(root, omitted_pack, schema):
        return ["self-test:omitted-pack-not-rejected"]
    extra_pack = deepcopy(manifest)
    extra_pack["packs"].append(deepcopy(extra_pack["packs"][0]))
    if not verify(root, extra_pack, schema):
        return ["self-test:extra-pack-not-rejected"]
    traversal_pack = deepcopy(manifest)
    traversal_pack["packs"][0]["path"] = "../pack.json"
    if not verify(root, traversal_pack, schema):
        return ["self-test:pack-traversal-not-rejected"]
    wrong_pack_hash = deepcopy(manifest)
    wrong_pack_hash["packs"][0]["sha256"] = "0" * 64
    if not any("input-identity:sha256" in error for error in verify(root, wrong_pack_hash, schema)):
        return ["self-test:pack-identity-not-rejected"]
    wrong_pack_length = deepcopy(manifest)
    wrong_pack_length["packs"][0]["byte_length"] += 1
    if not any("input-identity:length" in error for error in verify(root, wrong_pack_length, schema)):
        return ["self-test:pack-length-not-rejected"]

    absent_entry = deepcopy(manifest)
    item = _case(absent_entry, packed_case["case_id"])["inputs"][packed_index]
    item["location"]["entry_id"] = "release/oracle/vectors/release-state/absent/context.json"
    if not verify(root, absent_entry, schema):
        return ["self-test:absent-entry-not-rejected"]
    wrong_suite = deepcopy(manifest)
    item = _case(wrong_suite, packed_case["case_id"])["inputs"][packed_index]
    item["location"]["pack_id"] = "release-state"
    if not verify(root, wrong_suite, schema):
        return ["self-test:wrong-suite-pack-not-rejected"]
    wrong_entry_hash = deepcopy(manifest)
    _case(wrong_entry_hash, packed_case["case_id"])["inputs"][packed_index]["sha256"] = "0" * 64
    if not any("input-identity:sha256" in error for error in verify(root, wrong_entry_hash, schema)):
        return ["self-test:entry-identity-not-rejected"]
    wrong_entry_length = deepcopy(manifest)
    _case(wrong_entry_length, packed_case["case_id"])["inputs"][packed_index]["byte_length"] += 1
    if not any("input-identity:length" in error for error in verify(root, wrong_entry_length, schema)):
        return ["self-test:entry-length-not-rejected"]

    if not closure_self_test(payloads):
        return ["self-test:pack-closure-mutation-not-rejected"]
    return []


def self_test(root: Path, manifest: dict[str, Any], schema: dict[str, Any]) -> list[str]:
    payloads = load_packs(root, manifest, schema, [])
    profile_errors = _pack_profile_self_test(schema)
    if profile_errors:
        return profile_errors
    location_errors = _location_and_closure_self_test(root, manifest, schema, payloads)
    if location_errors:
        return location_errors
    filesystem_errors = _filesystem_self_test(root, manifest, schema)
    if filesystem_errors:
        return filesystem_errors
    wrong_digest = deepcopy(manifest)
    wrong_digest["cases"][0]["inputs"][0]["sha256"] = "0" * 64
    if not any(error.startswith("input-identity:sha256:") for error in verify(root, wrong_digest, schema)):
        return ["self-test:digest-drift-not-rejected"]

    duplicate = deepcopy(manifest)
    duplicate["cases"].insert(1, deepcopy(duplicate["cases"][0]))
    if "manifest-contract:case-order-or-duplicate" not in verify(root, duplicate, schema):
        return ["self-test:duplicate-case-not-rejected"]

    wrong_identity = deepcopy(manifest)
    wrong_identity["profile_contract_commit"] = "incorrect"
    if not verify(root, wrong_identity, schema):
        return ["self-test:identity-drift-not-rejected"]

    state_cases = [case for case in manifest["cases"] if case["suite_id"] == "release-state"]
    governed = next(case for case in state_cases if "governance" in case["fixture_case_id"])
    plain = next(case for case in state_cases if case["fixture_case_id"] == "development")
    omitted = deepcopy(manifest)
    governed_input = _case(omitted, governed["case_id"])["inputs"][0]
    governed_input.update(deepcopy(plain["inputs"][0]))
    if not any("state-context-case" in error for error in verify(root, omitted, schema)):
        return ["self-test:state-governance-omission-not-rejected"]

    sequence_case = next(case for case in manifest["cases"] if "sequence" in case)
    reused = deepcopy(manifest)
    target = _case(reused, sequence_case["case_id"])
    target["sequence"][1] = deepcopy(target["sequence"][0])
    if not any("sequence-" in error for error in verify(root, reused, schema)):
        return ["self-test:reused-prior-not-rejected"]

    reversed_members = deepcopy(manifest)
    _case(reversed_members, sequence_case["case_id"])["sequence"].reverse()
    if not any("sequence-" in error for error in verify(root, reversed_members, schema)):
        return ["self-test:reversed-sequence-not-rejected"]

    reused_input = deepcopy(manifest)
    target = _case(reused_input, sequence_case["case_id"])
    prior_index = next(index for index, item in enumerate(target["inputs"]) if item["role"] == "prior-snapshot")
    current_index = next(index for index, item in enumerate(target["inputs"]) if item["role"] == "current-snapshot")
    current_role, current_ordinal = target["inputs"][current_index]["role"], target["inputs"][current_index]["ordinal"]
    target["inputs"][current_index] = deepcopy(target["inputs"][prior_index])
    target["inputs"][current_index].update(role=current_role, ordinal=current_ordinal)
    if not verify(root, reused_input, schema):
        return ["self-test:reused-packed-sequence-input-not-rejected"]

    reversed_inputs = deepcopy(manifest)
    target = _case(reversed_inputs, sequence_case["case_id"])
    prior_index = next(index for index, item in enumerate(target["inputs"]) if item["role"] == "prior-snapshot")
    current_index = next(index for index, item in enumerate(target["inputs"]) if item["role"] == "current-snapshot")
    left, right = target["inputs"][prior_index], target["inputs"][current_index]
    left_identity = (deepcopy(left["location"]), left["byte_length"], left["sha256"])
    right_identity = (deepcopy(right["location"]), right["byte_length"], right["sha256"])
    left["location"], left["byte_length"], left["sha256"] = right_identity
    right["location"], right["byte_length"], right["sha256"] = left_identity
    if not verify(root, reversed_inputs, schema):
        return ["self-test:reversed-packed-sequence-inputs-not-rejected"]

    wrong_name = deepcopy(manifest)
    _case(wrong_name, sequence_case["case_id"])["sequence"][1]["current_name"] = "wrong.json"
    if not any("sequence-current-name" in error for error in verify(root, wrong_name, schema)):
        return ["self-test:sequence-name-disagreement-not-rejected"]

    coverage = strict_json_load(root / "release" / "oracle" / "coverage-inventory-v1.json")
    weakened = deepcopy(coverage)
    weakened["repair_regressions"].pop()
    errors: list[str] = []
    verify_coverage(root, manifest, payloads, errors, weakened)
    if "manifest-contract:coverage-exact" not in errors:
        return ["self-test:coverage-repair-omission-not-rejected"]

    weakened = deepcopy(coverage)
    weakened["subjects"][0]["case_keys"].pop()
    errors = []
    verify_coverage(root, manifest, payloads, errors, weakened)
    if "manifest-contract:coverage-exact" not in errors:
        return ["self-test:coverage-subject-omission-not-rejected"]

    swapped = deepcopy(coverage)
    swapped["transition_pairs"][0]["case_key"], swapped["transition_pairs"][1]["case_key"] = (
        swapped["transition_pairs"][1]["case_key"], swapped["transition_pairs"][0]["case_key"],
    )
    errors = []
    verify_coverage(root, manifest, payloads, errors, swapped)
    if "manifest-contract:coverage-exact" not in errors:
        return ["self-test:coverage-transition-swap-not-rejected"]
    return []
