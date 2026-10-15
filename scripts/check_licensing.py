#!/usr/bin/env python3
"""Lightweight licensing guard (D-033). Standard library only, offline.

Fails when:
- the root LICENSE is no longer the unmodified Apache License 2.0 text, or the
  PolyForm Noncommercial 1.0.0 text in LICENSES/ was altered;
- the workspace sets a default crate license (new crates would inherit it
  silently instead of being classified);
- a crate is missing from licensing.toml, or its Cargo.toml `license` differs
  from the map, or uses a license other than Apache-2.0 or
  PolyForm-Noncommercial-1.0.0;
- a PolyForm crate can be published to crates.io, lacks its LICENSE.md copy of
  the PolyForm text, or lacks a NOTICE with a `Required Notice:` line;
- an Apache-2.0 crate depends on a PolyForm crate (the open protocol and
  reference implementation must never need restricted code);
- the brand notice or LICENSING.md is missing, or LICENSING.md does not name
  every mapped crate.

This is a consistency check, not legal advice.
"""

import hashlib
import os
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(os.environ.get("SEPTUVO_ROOT") or Path(__file__).resolve().parents[1]).resolve()

APACHE = "Apache-2.0"
POLYFORM = "PolyForm-Noncommercial-1.0.0"
ALLOWED = {APACHE, POLYFORM}
# SHA-256 of the unmodified texts: the repository's LICENSE as published since
# 2026-10-13, and PolyForm Noncommercial 1.0.0 from polyformproject/polyform-licenses.
APACHE_SHA256 = "c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4"
POLYFORM_SHA256 = "c0ea4a896d2c8c394b29f9427589996db826cd501c512279ff0ed3ef48fabbe5"
POLYFORM_TEXT = "LICENSES/PolyForm-Noncommercial-1.0.0.md"


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_toml(path, errors):
    try:
        return tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as exc:
        errors.append(f"{path.name}: cannot read: {exc}")
        return None


def crate_dirs(root, errors):
    workspace = load_toml(root / "Cargo.toml", errors) or {}
    members = workspace.get("workspace", {}).get("members", [])
    return sorted(set(members)), workspace


def check(root=ROOT):
    errors = []

    if not (root / "LICENSE").is_file() or sha256(root / "LICENSE") != APACHE_SHA256:
        errors.append("LICENSE: must stay the unmodified Apache License 2.0 text (previously granted rights)")
    polyform = root / POLYFORM_TEXT
    if not polyform.is_file() or sha256(polyform) != POLYFORM_SHA256:
        errors.append(f"{POLYFORM_TEXT}: missing or not the official PolyForm Noncommercial 1.0.0 text")
    for required in ("LICENSING.md", "NOTICE", "licensing.toml"):
        if not (root / required).is_file():
            errors.append(f"{required}: required licensing file is missing")
    if errors:
        return errors

    mapping = load_toml(root / "licensing.toml", errors) or {}
    brand = mapping.get("brand", {})
    if brand and not (root / brand.get("notice", "")).is_file():
        errors.append(f"{brand.get('notice')}: brand rights notice is missing")
    mapped = {}
    for entry in mapping.get("crate", []):
        path, lic = entry.get("path", ""), entry.get("license", "")
        if lic not in ALLOWED:
            errors.append(f"licensing.toml {path}: license {lic!r} is not {APACHE} or {POLYFORM}")
        if path in mapped:
            errors.append(f"licensing.toml {path}: listed twice")
        mapped[path] = lic

    members, workspace = crate_dirs(root, errors)
    if "license" in workspace.get("workspace", {}).get("package", {}):
        errors.append("Cargo.toml: [workspace.package] must not set `license`; each crate declares its own")

    manifests = {}
    for member in members:
        manifest = load_toml(root / member / "Cargo.toml", errors)
        if manifest is None:
            continue
        manifests[member] = manifest
        package = manifest.get("package", {})
        lic = package.get("license")
        if member not in mapped:
            errors.append(f"{member}: crate is not classified in licensing.toml")
            continue
        if not isinstance(lic, str):
            errors.append(f"{member}/Cargo.toml: `license` must be set explicitly to {mapped[member]!r}")
        elif lic != mapped[member]:
            errors.append(f"{member}/Cargo.toml: license {lic!r} differs from licensing.toml ({mapped[member]!r})")
        if mapped[member] == POLYFORM:
            if package.get("publish") is not False:
                errors.append(f"{member}/Cargo.toml: PolyForm crates must set `publish = false`")
            copy = root / member / "LICENSE.md"
            if not copy.is_file() or sha256(copy) != POLYFORM_SHA256:
                errors.append(f"{member}/LICENSE.md: must be an unmodified copy of {POLYFORM_TEXT}")
            notice = root / member / "NOTICE"
            if not notice.is_file() or not re.search(r"^Required Notice: ", notice.read_text(encoding="utf-8"), re.M):
                errors.append(f"{member}/NOTICE: must contain a `Required Notice:` line")
    for path in mapped:
        if path not in members:
            errors.append(f"licensing.toml {path}: not a workspace member")

    # The open protocol and reference implementation must never need restricted code.
    by_dir = {m: mapped.get(m) for m in members}
    for member, manifest in manifests.items():
        if by_dir.get(member) != APACHE:
            continue
        for section in ("dependencies", "dev-dependencies", "build-dependencies"):
            for name, spec in manifest.get(section, {}).items():
                if isinstance(spec, dict) and "path" in spec:
                    target = (root / member / spec["path"]).resolve()
                    target_rel = target.relative_to(root).as_posix() if target.is_relative_to(root) else str(target)
                    if by_dir.get(target_rel) == POLYFORM:
                        errors.append(f"{member}: Apache-2.0 crate depends on PolyForm crate {name} ({target_rel})")

    licensing_md = (root / "LICENSING.md").read_text(encoding="utf-8")
    for path in mapped:
        if path not in licensing_md:
            errors.append(f"LICENSING.md: does not mention {path}")
    return errors


def main():
    errors = check()
    if errors:
        for e in errors:
            print(f"ERROR: {e}", file=sys.stderr)
        return 1
    print("PASS: licensing map, crate metadata, license texts and notices are consistent (not legal advice).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
