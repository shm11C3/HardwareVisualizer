#!/usr/bin/env python3
"""Create an isolated, version-pinned engine experiment environment on macOS."""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tarfile

SQLITE_VERSION = "3.46.0"
PYSQLITE_VERSION = "0.5.4"
DEFINES = [
    "SQLITE_THREADSAFE=1", "SQLITE_ENABLE_DBSTAT_VTAB",
    "SQLITE_ENABLE_COLUMN_METADATA", "SQLITE_ENABLE_STAT4",
    "SQLITE_ENABLE_API_ARMOR", "SQLITE_ENABLE_FTS5", "SQLITE_ENABLE_RTREE",
    "SQLITE_ENABLE_UNLOCK_NOTIFY", "SQLITE_MAX_VARIABLE_NUMBER=32766",
]


def run(command):
    subprocess.run([str(value) for value in command], check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sqlite-source", type=Path, required=True,
                        help="Directory containing SQLite 3.46.0 sqlite3.c and sqlite3.h")
    parser.add_argument("--output", type=Path, required=True,
                        help="New directory for the venv and build artifacts")
    args = parser.parse_args()
    if sys.platform != "darwin":
        parser.error("This setup script currently supports macOS only")
    source = args.sqlite_source.resolve()
    output = args.output.resolve()
    amalgamation = source / "sqlite3.c"
    header = source / "sqlite3.h"
    if not amalgamation.is_file() or not header.is_file():
        parser.error("--sqlite-source must contain sqlite3.c and sqlite3.h")
    if '#define SQLITE_VERSION        "3.46.0"' not in header.read_text():
        parser.error("SQLite source version must be 3.46.0")
    output.mkdir(parents=True, exist_ok=False)
    venv = output / "venv"
    run([sys.executable, "-m", "venv", venv])
    python = venv / "bin" / "python"
    library_dir = output / "sqlite-3.46.0"
    library_dir.mkdir()
    library = library_dir / "libsqlite3.0.dylib"
    compiler_command = ["cc", "-O2", "-dynamiclib"]
    compiler_command += ["-D" + value for value in DEFINES]
    compiler_command += [str(amalgamation), "-o", str(library)]
    # An absolute output path also gives the dylib an absolute install name.
    # Finish linking before building the binding, which records that name.
    run(compiler_command)
    (library_dir / "libsqlite3.dylib").symlink_to(library.name)
    requirements = Path(__file__).with_name("requirements.txt")
    run([python, "-m", "pip", "install", "--disable-pip-version-check",
         "-r", requirements])
    run([python, "-m", "pip", "download", "--no-deps", "--no-binary=pysqlite3",
         "--disable-pip-version-check", "-d", output,
         "pysqlite3==" + PYSQLITE_VERSION])
    package = output / ("pysqlite3-" + PYSQLITE_VERSION + ".tar.gz")
    with tarfile.open(package) as archive:
        archive.extractall(output, filter="data")
    binding_source = output / ("pysqlite3-" + PYSQLITE_VERSION)
    # pysqlite3 overwrites CFLAGS on macOS; build_ext is its supported path knob.
    (binding_source / "setup.cfg").write_text(
        "[build_ext]\ninclude_dirs=" + str(source)
        + "\nlibrary_dirs=" + str(library_dir) + "\n"
    )
    run([python, "-m", "pip", "install", "--disable-pip-version-check",
         "--no-cache-dir", binding_source])
    verification = subprocess.check_output([
        str(python), "-c",
        "import json,pysqlite3,duckdb; "
        "assert pysqlite3.sqlite_version == '3.46.0'; "
        "assert duckdb.__version__ == '1.5.5'; "
        "print(json.dumps({'sqlite':pysqlite3.sqlite_version,'duckdb':duckdb.__version__}))",
    ], cwd=output, text=True)
    metadata = {
        "versions": json.loads(verification),
        "python": sys.version,
        "sqlite_source_sha256": hashlib.sha256(amalgamation.read_bytes()).hexdigest(),
        "sqlite_header_sha256": hashlib.sha256(header.read_bytes()).hexdigest(),
        "pysqlite_source_sha256": hashlib.sha256(package.read_bytes()).hexdigest(),
        "compiler_command": compiler_command,
        "compiler": subprocess.check_output(["cc", "--version"], text=True),
        "binding": "pysqlite3==" + PYSQLITE_VERSION,
        "scope": "Python client experiment; not the production SQLx build or Tauri bundle",
    }
    (output / "setup.json").write_text(json.dumps(metadata, indent=2) + "\n")
    print("Ready:", python)


if __name__ == "__main__":
    main()
