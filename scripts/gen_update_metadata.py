#!/usr/bin/env python3
"""Generate electron-updater YAML manifests and SHA-256 checksum files for Glyph releases."""

from __future__ import annotations

import argparse
import base64
from datetime import datetime, timezone
import hashlib
from pathlib import Path


def file_hashes(path: Path) -> tuple[str, str, int]:
    """Return (base64_sha512, hex_sha256, size_bytes) streamed in 1 MiB chunks."""
    h512 = hashlib.sha512()
    h256 = hashlib.sha256()
    size = 0
    with path.open("rb") as f:
        while chunk := f.read(1024 * 1024):
            h512.update(chunk)
            h256.update(chunk)
            size += len(chunk)
    return base64.b64encode(h512.digest()).decode("ascii"), h256.hexdigest(), size


def build_manifest(
    version: str,
    primary: Path,
    extras: list[Path],
    notes_path: Path | None,
) -> str:
    """Format an electron-updater compatible latest*.yml manifest."""
    entries = []
    for item in [primary, *extras]:
        sha512_b64, _, size = file_hashes(item)
        entries.append((item.name, sha512_b64, size))

    primary_name, primary_sha512, _ = entries[0]
    release_date = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.000Z")

    lines = [
        f"version: {version}",
        "files:",
    ]
    for name, sha512_b64, size in entries:
        lines.extend(
            [
                f"  - url: {name}",
                f"    sha512: {sha512_b64}",
                f"    size: {size}",
            ]
        )
    lines.extend(
        [
            f"path: {primary_name}",
            f"sha512: {primary_sha512}",
        ]
    )

    if notes_path and notes_path.is_file():
        notes_raw = notes_path.read_text(encoding="utf-8").replace("\r\n", "\n").strip("\n")
        if notes_raw:
            lines.append("releaseNotes: |")
            for note_line in notes_raw.split("\n"):
                lines.append(f"  {note_line}" if note_line else "")

    lines.append(f"releaseDate: '{release_date}'")
    return "\n".join(lines) + "\n"


def build_sha256sums(paths: list[Path]) -> str:
    """Format standard two-space `<sha256>  <filename>` lines sorted by basename."""
    rows = []
    for p in sorted(paths, key=lambda x: x.name):
        _, sha256_hex, _ = file_hashes(p)
        rows.append(f"{sha256_hex}  {p.name}")
    return "\n".join(rows) + ("\n" if rows else "")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, help="Output path for latest*.yml")
    parser.add_argument("--version", default="3.0.0", help="Semantic version without leading v")
    parser.add_argument("--primary", type=Path, help="Primary update asset path")
    parser.add_argument("--extra", type=Path, action="append", default=[], help="Additional asset")
    parser.add_argument("--notes", type=Path, help="Markdown release notes file")
    parser.add_argument("--sha256-out", type=Path, help="Output path for SHA-256 summary file")
    parser.add_argument("--sha256-files", type=Path, nargs="*", default=[], help="Files to hash")
    args = parser.parse_args()

    if args.manifest:
        if not args.primary:
            parser.error("--primary is required when --manifest is specified")
        manifest_text = build_manifest(
            version=args.version.lstrip("vV"),
            primary=args.primary,
            extras=args.extra,
            notes_path=args.notes,
        )
        args.manifest.parent.mkdir(parents=True, exist_ok=True)
        args.manifest.write_text(manifest_text, encoding="utf-8", newline="\n")

    if args.sha256_out:
        sums_text = build_sha256sums(args.sha256_files)
        args.sha256_out.parent.mkdir(parents=True, exist_ok=True)
        args.sha256_out.write_text(sums_text, encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()
