#!/usr/bin/env python3
"""Author the reviewed oracle candidate; never run this in release gates."""

from copy import deepcopy
from hashlib import sha256
from pathlib import Path
import json
import shutil
import sys

sys.dont_write_bytecode = True

RELEASE = Path(__file__).resolve().parents[1]
REPO = RELEASE.parent
sys.path.insert(0, str(RELEASE))

from policy_check.observation import observe
from policy_check.runner import _fixture_bytes, _load, _load_document, _mutate
from policy_check.evidence import observed_digest
from policy_check.common import strict_json_loads
from coverage_contract import PACK_PATHS, expected_inventory, oracle_id

ORACLE = RELEASE / "oracle"
VECTORS = ORACLE / "vectors"
EXPECTED_OUTPUT = ORACLE / "python-observations-v1.json"
SCHEMA = RELEASE / "schemas" / "release-evidence-v1.schema.json"
REASON_MAP = ORACLE / "reason-map-v1.json"
PAYLOADS: dict[str, bytes] = {}

TABLES = {
    "signer-authority": "release/fixtures/signer-authority-cases.json",
    "signer-governance": "release/fixtures/signer-governance-cases.json",
    "signer-challenge": "release/fixtures/signer-challenge-cases.json",
    "release-state": "release/fixtures/release-state-cases.json",
    "schema-evaluator": "release/fixtures/schema-evaluator-cases.json",
}


def write_bytes(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(content)


def json_bytes(value: object) -> bytes:
    return (json.dumps(value, indent=2, ensure_ascii=False) + "\n").encode()


def identity(path: str) -> dict[str, object]:
    content = (REPO / path).read_bytes()
    return {"path": path, "byte_length": len(content), "sha256": sha256(content).hexdigest()}


def add_payload(path: str, content: bytes) -> None:
    content.decode("utf-8")
    if path in PAYLOADS:
        raise ValueError(f"duplicate vector path: {path}")
    PAYLOADS[path] = content


def input_for(path: str, role: str, ordinal: int) -> dict[str, object]:
    if path in PAYLOADS:
        pack_id = path.split("/")[3]
        content = PAYLOADS[path]
        location = {"kind": "packed", "pack_id": pack_id, "entry_id": path}
        input_identity = {"byte_length": len(content), "sha256": sha256(content).hexdigest()}
    else:
        location = {"kind": "direct", "path": path}
        input_identity = identity(path)
        del input_identity["path"]
    return {"role": role, "ordinal": ordinal, "location": location, **input_identity}


def write_packs() -> list[dict[str, object]]:
    registry = []
    for pack_id, path in sorted(PACK_PATHS.items()):
        prefix = f"release/oracle/vectors/{pack_id}/"
        entries = [
            {"entry_id": entry_id, "content": PAYLOADS[entry_id].decode("utf-8")}
            for entry_id in sorted(PAYLOADS) if entry_id.startswith(prefix)
        ]
        write_bytes(REPO / path, json_bytes({"schema_version": "oracle-vector-pack-v1", "entries": entries}))
        registry.append({"pack_id": pack_id, **identity(path)})
    return registry


def expected(record: dict[str, str], reason_map: dict[str, str]) -> dict[str, str]:
    structural = record.get("structural", "not-run")
    semantic = record.get("semantic", "not-run")
    if record["suite_id"] in {"json-parser", "schema-evaluator"}:
        structural = record["final"]
    elif record["suite_id"] != "release-evidence":
        semantic = record["final"]
    return {
        "structural": structural,
        "semantic": semantic,
        "final": record["final"],
        "case_outcome": record["case_outcome"],
        "primary_reason": (
            "none" if record["final"] == "valid"
            else reason_map[f"{record['suite_id']}:{record['case_id']}"]
        ),
    }


def release_state_materializations() -> dict[str, list[tuple[str, str]]]:
    result = {}
    for case in _load(RELEASE / "fixtures" / "release-state-cases.json")["cases"]:
        governance = case.get("governance")
        resolved: dict[str, object] | None = None
        if isinstance(governance, dict):
            reference = governance.get("hold_evidence")
            source = RELEASE / "fixtures" / f"release-evidence-{reference}.json"
            if source.exists():
                resolved = {
                    "state": "present", "reference": reference,
                    "source_path": source.relative_to(REPO).as_posix(),
                    "document": _load(source),
                }
            else:
                resolved = {"state": "absent", "reference": reference}
        path = f"release/oracle/vectors/release-state/{case['id']}/context.json"
        add_payload(path, json_bytes({"case": case, "governance_evidence": resolved}))
        result[case["id"]] = [(path, "fixture-table")]
    return result


def materialize_challenges() -> dict[str, list[tuple[str, str]]]:
    result = {}
    table = _load(RELEASE / "fixtures" / "signer-challenge-cases.json")
    for case in table["cases"]:
        base = f"release/oracle/vectors/signer-challenge/{case['id']}"
        challenge_path = f"{base}/challenge.txt"
        context_path = f"{base}/context.json"
        add_payload(challenge_path, case["challenge"].encode())
        context = {key: value for key, value in case.items() if key not in {"challenge", "expected"}}
        add_payload(context_path, json_bytes(context))
        result[case["id"]] = [(context_path, "fixture-table"), (challenge_path, "challenge")]
    return result


def evidence_materializations() -> dict[str, tuple[list[tuple[str, str]], list[dict[str, object]]]]:
    fixtures = RELEASE / "fixtures"
    names = {
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
    bases = {name: _load_document(fixtures / file) for name, file in names.items()}
    result = {}
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
        prior = None
        prior_bytes = None
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
        base = f"release/oracle/vectors/release-evidence/{case['id']}"
        parsed_path = f"{base}/parsed.json"
        current_path = f"{base}/current.json"
        add_payload(parsed_path, json_bytes({"prior": prior, "current": current}))
        add_payload(current_path, current_bytes)
        paths = [(parsed_path, "fixture-table")]
        if prior_bytes is not None:
            prior_path = f"{base}/prior.json"
            add_payload(prior_path, prior_bytes)
            paths.append((prior_path, "prior-snapshot"))
        paths.append((current_path, "current-snapshot"))
        result[case["id"]] = (paths, [])
    return result


def build() -> None:
    PAYLOADS.clear()
    observations = observe(RELEASE)
    write_bytes(EXPECTED_OUTPUT, json_bytes(observations))
    challenges = materialize_challenges()
    states = release_state_materializations()
    evidence = evidence_materializations()
    packs = write_packs()
    reason_map = _load(REASON_MAP)
    output_path = "release/oracle/python-observations-v1.json"
    cases = []
    for record in observations["cases"]:
        suite, fixture_case_id = record["suite_id"], record["case_id"]
        case_id = oracle_id(fixture_case_id)
        paths: list[tuple[str, str]] = []
        if suite == "signer-authority-live":
            paths.append(("release-signers.toml", "authority"))
        elif suite == "json-parser":
            table = _load(RELEASE / "fixtures" / "json-parser-cases.json")
            source = next(case["path"] for case in table["cases"] if case["id"] == fixture_case_id)
            paths.append((f"release/{source}", "fixture-table"))
        elif suite == "signer-challenge":
            paths.extend(challenges[fixture_case_id])
        elif suite == "release-evidence":
            paths.append(("release/schemas/release-evidence-v1.schema.json", "schema"))
            paths.extend(evidence[fixture_case_id][0])
        elif suite == "release-state":
            paths.extend(states[fixture_case_id])
            paths.append(("release/schemas/release-evidence-v1.schema.json", "schema"))
        else:
            paths.append((TABLES[suite], "fixture-table"))
        paths.append((output_path, "expected-output"))
        inputs = [input_for(path, role, index) for index, (path, role) in enumerate(paths)]
        case_record = {
            "suite_id": suite,
            "case_id": case_id,
            "fixture_case_id": fixture_case_id,
            "inputs": inputs,
            "expected": expected(record, reason_map),
        }
        if suite == "release-evidence" and any(item["role"] == "prior-snapshot" for item in inputs):
            prior_input = next(item for item in inputs if item["role"] == "prior-snapshot")
            current_input = next(item for item in inputs if item["role"] == "current-snapshot")
            prior_path = prior_input["location"]["entry_id"]
            current_path = current_input["location"]["entry_id"]
            prior = strict_json_loads(PAYLOADS[prior_path])
            current = strict_json_loads(PAYLOADS[current_path])
            prior_name = f"prikk-{prior['version']}-release-evidence-{prior['sequence']}.json"
            current_name = f"prikk-{current['version']}-release-evidence-{current['sequence']}.json"
            case_record["sequence"] = [
                {
                    "input_ordinal": prior_input["ordinal"],
                    "predecessor_name": None,
                    "current_name": prior_name,
                    "byte_length": prior_input["byte_length"],
                    "sha256": prior_input["sha256"],
                },
                {
                    "input_ordinal": current_input["ordinal"],
                    "predecessor_name": prior_name,
                    "current_name": current_name,
                    "byte_length": current_input["byte_length"],
                    "sha256": current_input["sha256"],
                },
            ]
        cases.append(case_record)
    manifest = {
        "schema_version": "oracle-manifest-v1",
        "python_baseline_commit": "12c137d",
        "profile_contract_commit": "ea427df",
        "observation_adapter_commit": "6be65af",
        "reason_taxonomy_version": 1,
        "reason_map": identity("release/oracle/reason-map-v1.json"),
        "normative_schema": identity("release/schemas/release-evidence-v1.schema.json"),
        "packs": packs,
        "cases": cases,
    }
    write_bytes(ORACLE / "oracle-manifest-v1.json", json_bytes(manifest))
    payload_map = {(path.split("/")[3], path): content for path, content in PAYLOADS.items()}
    write_bytes(
        ORACLE / "coverage-inventory-v1.json",
        json_bytes(expected_inventory(REPO, manifest, payload_map)),
    )
    if VECTORS.exists():
        shutil.rmtree(VECTORS)


if __name__ == "__main__":
    build()
