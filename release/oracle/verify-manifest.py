#!/usr/bin/env python3
"""Verify the frozen release-policy oracle manifest without evaluating policy semantics."""

from pathlib import Path
import argparse
import json
import sys

sys.dont_write_bytecode = True

ORACLE = Path(__file__).resolve().parent
RELEASE = ORACLE.parent
ROOT = RELEASE.parent
sys.path.insert(0, str(RELEASE))

from manifest_self_test import self_test
from manifest_verify import verify
from policy_check.common import strict_json_load


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--format", choices=["json"], default="json")
    parser.add_argument("--self-test", action="store_true")
    arguments = parser.parse_args()
    manifest = strict_json_load(ORACLE / "oracle-manifest-v1.json")
    schema = strict_json_load(ORACLE / "oracle-manifest-v1.schema.json")
    errors = self_test(ROOT, manifest, schema) if arguments.self_test else verify(ROOT, manifest, schema)
    result = {
        "schema_version": "oracle-verification-result-v1",
        "valid": not errors,
        "case_count": len(manifest.get("cases", [])),
        "errors": errors,
    }
    json.dump(result, sys.stdout, indent=2)
    sys.stdout.write("\n")
    sys.exit(1 if errors else 0)
