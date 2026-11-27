#!/usr/bin/env python3
"""Run a build with disposable scratch space and protect disk headroom."""

import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
from tempfile import TemporaryDirectory


MIB = 1024 * 1024
RESERVE_MIB = 2048


def require_space(locations, required_mib):
    for location in dict.fromkeys(locations):
        free_mib = shutil.disk_usage(location).free // MIB
        if free_mib < required_mib:
            raise RuntimeError(
                f"{location} has {free_mib} MiB free. This build needs "
                f"{required_mib} MiB to protect disk headroom."
            )


def run_build(command, environment, locations):
    with subprocess.Popen(command, env=environment, start_new_session=True) as build:
        try:
            while True:
                try:
                    return build.wait(timeout=1)
                except subprocess.TimeoutExpired:
                    require_space(locations, RESERVE_MIB)
        finally:
            if build.poll() is None:
                try:
                    os.killpg(build.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                build.wait()


def interrupt(_signum, _frame):
    raise KeyboardInterrupt


def main(output, estimate_mib, command):
    environment = os.environ.copy()
    output = Path(output).resolve()
    explicit_target = environment.get("CARGO_TARGET_DIR")
    locations = [output]
    if explicit_target:
        target = Path(explicit_target).resolve()
        environment["CARGO_TARGET_DIR"] = str(target)
        locations.append(next(p for p in (target, *target.parents) if p.exists()))
    require_space(locations, RESERVE_MIB + estimate_mib)
    try:
        with TemporaryDirectory(prefix=".scenic-build-", dir=output) as scratch:
            environment["TMPDIR"] = scratch
            environment["SCENIC_BUILD_SCRATCH"] = scratch
            environment.setdefault("CARGO_TARGET_DIR", str(Path(scratch) / "target"))
            return run_build(command, environment, locations)
    finally:
        free = shutil.disk_usage(output).free / (1024 * MIB)
        print(f"Scenic Flight build scratch released. {free:.1f} GiB available.", file=sys.stderr)


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, interrupt)
    try:
        sys.exit(main(sys.argv[1], int(sys.argv[2]), sys.argv[3:]))
    except KeyboardInterrupt:
        sys.exit(130)
    except (OSError, RuntimeError) as error:
        sys.exit(f"Scenic Flight build stopped. {error}")
