#!/usr/bin/env python3
"""Run the tracked DC-35 release-policy fixture audit."""

from pathlib import Path
import sys

sys.dont_write_bytecode = True

from policy_check.runner import run


if __name__ == "__main__":
    sys.exit(run(Path(__file__).resolve().parent))
