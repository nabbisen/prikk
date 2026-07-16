#!/usr/bin/env python3
"""Emit deterministic per-case observations from the Python release policy."""

from pathlib import Path
import argparse
import json
import sys

sys.dont_write_bytecode = True

from policy_check.observation import observe
from policy_check.observation_verify import self_test, verify_observations


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--self-test", action="store_true")
    arguments = parser.parse_args()
    root = Path(__file__).resolve().parent
    if arguments.check or arguments.self_test:
        errors = self_test(root) if arguments.self_test else verify_observations(root)
        if errors:
            for error in errors:
                print(f"FAIL: {error}")
            sys.exit(1)
        if arguments.self_test:
            print("python policy observations: negative self-test passed")
        else:
            print("python policy observations: all fixture outcomes matched")
    else:
        json.dump(observe(root), sys.stdout, indent=2)
        sys.stdout.write("\n")
