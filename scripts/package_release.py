#!/usr/bin/env python3
"""Package the tested Windows executable and pinned upstream tools."""

import argparse
import hashlib
import io
import re
import zipfile
from pathlib import Path


TOOTHTRAY_SHA256 = "0c1b90761654d61dcd52cba23d1a82e5e576dcd4c6fa678618b815ff113e75d9"
SOUNDVOLUMEVIEW_SHA256 = "ee2c45553fb9fb31b71db88b537c18e25c1a387a4a9009d081bdb38265c68e1f"
SOUNDVOLUMEVIEW_FILES = {"SoundVolumeView.exe", "SoundVolumeView.chm", "readme.txt"}
BUILD_PATH = re.compile(rb"(?i)(?:/home/|/users/|[a-z]:\\users\\|\\home\\)")


def checked_exe(path: Path) -> bytes:
    data = path.read_bytes()
    if data[:2] != b"MZ":
        raise ValueError("Windows EXE is missing or invalid")
    decoded = (data[offset:].decode("utf-16le", errors="ignore").encode("utf-8") for offset in (0, 1))
    if BUILD_PATH.search(data) or any(BUILD_PATH.search(text) for text in decoded):
        raise ValueError("Windows EXE contains an absolute user build path")
    return data


def verified_bytes(path: Path, expected_sha256: str) -> bytes:
    data = path.read_bytes()
    actual = hashlib.sha256(data).hexdigest()
    if actual != expected_sha256:
        raise ValueError(f"SHA-256 mismatch for {path}: {actual}")
    return data


def add_file(archive: zipfile.ZipFile, destination: str, data: bytes) -> None:
    info = zipfile.ZipInfo(f"EarbudsPC/{destination}", date_time=(2026, 10, 17, 0, 0, 0))
    info.compress_type = zipfile.ZIP_DEFLATED
    info.external_attr = 0o644 << 16
    archive.writestr(info, data)


def package(exe: Path, toothtray_zip: Path, soundvolumeview_zip: Path, output: Path) -> None:
    executable = checked_exe(exe)
    toothtray = verified_bytes(toothtray_zip, TOOTHTRAY_SHA256)
    soundvolumeview = verified_bytes(soundvolumeview_zip, SOUNDVOLUMEVIEW_SHA256)
    with zipfile.ZipFile(io.BytesIO(toothtray)) as upstream:
        bluetooth = upstream.read("x64Release/ToothTrayCLI.exe")
    with zipfile.ZipFile(io.BytesIO(soundvolumeview)) as upstream:
        if set(upstream.namelist()) != SOUNDVOLUMEVIEW_FILES:
            raise ValueError("SoundVolumeView archive contents changed")
        sound_files = {name: upstream.read(name) for name in sorted(SOUNDVOLUMEVIEW_FILES)}

    root = Path(__file__).resolve().parent.parent
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, "w") as archive:
        add_file(archive, "EarbudsPC.exe", executable)
        add_file(archive, "README.txt", (root / "README-release.txt").read_bytes())
        add_file(archive, "tools/ToothTrayCLI.exe", bluetooth)
        add_file(archive, "tools/ToothTrayCLI-LICENSE.md", (root / "third_party/ToothTrayCLI-LICENSE.md").read_bytes())
        add_file(archive, "tools/Rust-Crates-LICENSES.txt", (root / "third_party/Rust-Crates-LICENSES.txt").read_bytes())
        for name, data in sound_files.items():
            add_file(archive, f"tools/SoundVolumeView/{name}", data)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--exe", type=Path, required=True)
    parser.add_argument("--toothtray-zip", type=Path, required=True)
    parser.add_argument("--soundvolumeview-zip", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    package(args.exe, args.toothtray_zip, args.soundvolumeview_zip, args.output)
