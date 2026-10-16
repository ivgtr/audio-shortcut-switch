#!/usr/bin/env python3
"""Generate notices from the locked Windows Cargo dependency resolution."""

import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "third_party" / "Rust-Crates-LICENSES.txt"
TARGET = "x86_64-pc-windows-msvc"
NOTICE_NAME = re.compile(r"^(?:license|licence|copying|notice)", re.IGNORECASE)
SLINT_LICENSE = "LicenseRef-Slint-Royalty-free-2.0.md"


def license_files(package: dict) -> list[Path]:
    package_root = Path(package["manifest_path"]).parent
    files = []
    for entry in package_root.iterdir():
        if not NOTICE_NAME.match(entry.name):
            continue
        if entry.is_file():
            files.append(entry)
        elif entry.is_dir():
            files.extend(path for path in entry.rglob("*") if path.is_file())
    if package["name"] == "slint" or package["name"].startswith("i-slint-") or package["name"] in {"slint-build", "slint-macros"}:
        files = [path for path in files if path.name == SLINT_LICENSE]
        if len(files) != 1:
            raise ValueError(f"Expected one Slint royalty-free license file for {package['name']}")
    return sorted(files)


def generate() -> str:
    result = subprocess.run(
        ["cargo", "metadata", "--locked", "--filter-platform", TARGET, "--format-version", "1"],
        cwd=ROOT,
        capture_output=True,
        check=True,
    )
    metadata = json.loads(result.stdout)
    root_id = metadata["resolve"]["root"]
    app = next(package for package in metadata["packages"] if package["id"] == root_id)
    packages = sorted(
        (package for package in metadata["packages"] if package["id"] != root_id),
        key=lambda package: (package["name"], package["version"]),
    )
    documents = {}
    package_lines = []
    for package in packages:
        if not package["source"] or not package["source"].startswith("registry+"):
            raise ValueError(f"Unexpected dependency source for {package['name']}: {package['source']}")
        license_expression = package["license"]
        if not license_expression:
            raise ValueError(f"Missing license metadata for {package['name']} {package['version']}")
        source_url = package["repository"] or f"https://crates.io/crates/{package['name']}/{package['version']}"
        document_ids = []
        for path in license_files(package):
            content = "\n".join(line.rstrip() for line in path.read_text(encoding="utf-8").splitlines()).strip() + "\n"
            digest = hashlib.sha256(content.encode("utf-8")).hexdigest()[:12]
            if digest in documents and documents[digest] != content:
                raise ValueError(f"License document identifier collision: {digest}")
            documents[digest] = content
            document_ids.append(digest)
        package_lines.extend(
            [
                f"{package['name']} {package['version']} | {license_expression}",
                source_url,
                "Bundled license documents: " + (", ".join(sorted(set(document_ids))) or "none in crate archive"),
                "",
            ]
        )
    lines = [
        f"Rust dependency notices for Audio Shortcut Switch {app['version']}",
        "",
        f"Source: cargo metadata --locked --filter-platform {TARGET}",
        "This list covers the resolved Windows dependencies, including build-time packages.",
        "License expressions and repository URLs come from each crate's Cargo metadata.",
        "License documents below are copied from the crate archives and deduplicated by content.",
        "Some crate archives do not contain a license document; their metadata is listed above.",
        "Slint components use the Slint Royalty-free Desktop, Mobile, and Web Applications License 2.0.",
        "The required attribution badge appears on the public project README and documentation site.",
        "https://slint.dev/agreements/slint-royalty-free-license.pdf",
        "",
        "Packages",
        "========",
        "",
        *package_lines,
        "License documents",
        "=================",
        "",
    ]
    for digest, content in sorted(documents.items()):
        lines.extend([f"--- {digest} ---", content.rstrip(), ""])
    return "\n".join(lines).rstrip() + "\n"


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Fail if the checked-in notices are stale")
    args = parser.parse_args()
    generated = generate()
    if args.check:
        if OUTPUT.read_text(encoding="utf-8") != generated:
            raise SystemExit(f"Outdated notices: run python scripts/generate_rust_licenses.py and commit {OUTPUT}")
    else:
        OUTPUT.write_text(generated, encoding="utf-8")
