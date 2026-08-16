#!/usr/bin/env python3
"""Generate the Jackdaw Bevy-coupled dependency inventory from Cargo metadata.

The classifier intentionally separates two concepts that otherwise collide:

* A dependency is Bevy-named when its package is `bevy` or starts with
  `bevy_`, matching the integration brief's coupling rule.
* A package is an official Bevy package only when its name is present in the
  checked-out local Bevy workspace metadata. This keeps third-party plugins
  such as `bevy_enhanced_input` in the vendoring closure.

Only packages reachable through resolved dependency edges in at least one
declared matrix are included. Registry checksums come from the pinned lockfile.
The generated files contain no timestamps or machine-specific source paths so
identical inputs produce identical output.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
import tomllib
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable
from urllib.parse import parse_qs, urlsplit, urlunsplit


Json = dict[str, Any]


@dataclass(frozen=True)
class MatrixSpec:
    label: str
    root_mode: str
    path: Path


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_json(path: Path) -> Json:
    with path.open("r", encoding="utf-8-sig") as source:
        value = json.load(source)
    if not isinstance(value, dict):
        raise ValueError(f"expected a JSON object in {path}")
    return value


def parse_matrix(value: str) -> MatrixSpec:
    descriptor, separator, raw_path = value.partition("=")
    if not separator:
        raise argparse.ArgumentTypeError(
            "matrix must use LABEL:MODE=PATH (MODE is root or workspace)"
        )
    label, separator, root_mode = descriptor.rpartition(":")
    if not separator or not label:
        raise argparse.ArgumentTypeError(
            "matrix must use LABEL:MODE=PATH (MODE is root or workspace)"
        )
    if root_mode not in {"root", "workspace"}:
        raise argparse.ArgumentTypeError("matrix MODE must be root or workspace")
    path = Path(raw_path).expanduser().resolve()
    return MatrixSpec(label=label, root_mode=root_mode, path=path)


def is_bevy_named(name: str) -> bool:
    return name == "bevy" or name.startswith("bevy_")


def normalized_path(path: str | Path) -> str:
    return os.path.normcase(os.path.abspath(os.fspath(path)))


def path_is_within(path: str | Path, root: str | Path) -> bool:
    candidate = normalized_path(path)
    parent = normalized_path(root)
    try:
        return os.path.commonpath((candidate, parent)) == parent
    except ValueError:
        return False


def official_bevy_names(bevy_metadata: Json, bevy_root: Path) -> set[str]:
    names: set[str] = set()
    for package in bevy_metadata.get("packages", []):
        name = package.get("name", "")
        manifest_path = package.get("manifest_path")
        if (
            is_bevy_named(name)
            and manifest_path
            and path_is_within(manifest_path, bevy_root)
        ):
            names.add(name)
    if "bevy" not in names:
        raise ValueError("local Bevy metadata did not identify the root `bevy` package")
    return names


def resolved_roots(metadata: Json, mode: str) -> list[str]:
    resolve = metadata.get("resolve")
    if not isinstance(resolve, dict):
        raise ValueError("Cargo metadata is missing its resolve graph")
    if mode == "root":
        root = resolve.get("root")
        if not root:
            raise ValueError("root-mode matrix has no Cargo resolve root")
        return [root]
    roots = metadata.get("workspace_members", [])
    if not roots:
        raise ValueError("workspace-mode matrix has no workspace members")
    return sorted(roots)


def reachable_package_ids(metadata: Json, roots: Iterable[str]) -> set[str]:
    resolve = metadata["resolve"]
    nodes = {node["id"]: node for node in resolve.get("nodes", [])}
    pending = list(roots)
    reached: set[str] = set()
    while pending:
        package_id = pending.pop()
        if package_id in reached:
            continue
        reached.add(package_id)
        node = nodes.get(package_id)
        if node is None:
            raise ValueError(f"resolve graph has no node for reachable package {package_id}")
        pending.extend(dependency["pkg"] for dependency in node.get("deps", []))
    return reached


def dependency_kinds(edge: Json) -> list[Json]:
    result = []
    for kind in edge.get("dep_kinds", []):
        result.append(
            {
                "kind": kind.get("kind") or "normal",
                "target": kind.get("target"),
            }
        )
    return sorted(result, key=lambda item: (item["kind"], item["target"] or ""))


def declared_couplings(package: Json) -> list[Json]:
    couplings = []
    for dependency in package.get("dependencies", []):
        if not is_bevy_named(dependency.get("name", "")):
            continue
        couplings.append(
            {
                "package": dependency["name"],
                "alias": dependency.get("rename"),
                "requirement": dependency.get("req"),
                "kind": dependency.get("kind") or "normal",
                "optional": bool(dependency.get("optional")),
                "default_features": bool(dependency.get("uses_default_features")),
                "features": sorted(dependency.get("features", [])),
                "target": dependency.get("target"),
            }
        )
    return sorted(
        couplings,
        key=lambda item: (
            item["package"],
            item["alias"] or "",
            item["kind"],
            item["target"] or "",
        ),
    )


def load_lock_index(lockfile: Path) -> tuple[dict[tuple[str, str, str], Json], str]:
    with lockfile.open("rb") as source:
        data = tomllib.load(source)
    index: dict[tuple[str, str, str], Json] = {}
    for package in data.get("package", []):
        key = (
            package["name"],
            package["version"],
            package.get("source", ""),
        )
        index[key] = package
    return index, sha256(lockfile)


def lock_record(package: Json, lock_index: dict[tuple[str, str, str], Json]) -> Json:
    key = (package["name"], package["version"], package.get("source") or "")
    record = lock_index.get(key)
    if record is not None:
        return record
    matches = [
        value
        for (name, version, _source), value in lock_index.items()
        if name == package["name"] and version == package["version"]
    ]
    if len(matches) == 1:
        return matches[0]
    raise ValueError(f"could not uniquely match {package['id']} in Cargo.lock")


def git_source_details(source: str) -> tuple[str, str | None, str | None]:
    without_prefix = source.removeprefix("git+")
    before_fragment, separator, revision = without_prefix.partition("#")
    parsed = urlsplit(before_fragment)
    query = parse_qs(parsed.query)
    repository = urlunsplit((parsed.scheme, parsed.netloc, parsed.path, "", ""))
    original_ref = None
    for key in ("rev", "tag", "branch"):
        values = query.get(key)
        if values:
            original_ref = values[0]
            break
    return repository.rstrip("/"), original_ref, revision if separator else None


def normalized_repository(repository: str) -> str:
    value = repository.strip().rstrip("/")
    if value.endswith(".git"):
        value = value[:-4]
    return value.lower()


def upstream_key(package: Json) -> tuple[str, str, str | None, str | None, str]:
    source = package.get("source") or ""
    if source.startswith("git+"):
        repository, original_ref, revision = git_source_details(source)
        return (
            "git:" + normalized_repository(repository),
            repository,
            original_ref,
            revision,
            "git-snapshot",
        )
    repository = package.get("repository")
    if repository:
        repository = repository.rstrip("/")
        return (
            "registry-repository:" + normalized_repository(repository),
            repository,
            None,
            None,
            "crates-io-snapshot",
        )
    repository = f"https://crates.io/crates/{package['name']}"
    return (
        f"registry-package:{package['name'].lower()}",
        repository,
        None,
        None,
        "crates-io-snapshot",
    )


def slug_base(repository: str, fallback: str) -> str:
    path = urlsplit(repository).path.rstrip("/")
    name = path.rsplit("/", 1)[-1] if path else fallback
    name = name.removesuffix(".git")
    slug = re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-")
    return slug or fallback.lower().replace("_", "-")


def allocate_slugs(groups: dict[str, Json]) -> dict[str, str]:
    bases: dict[str, list[str]] = defaultdict(list)
    for key, group in groups.items():
        bases[slug_base(group["repository"], group["packages"][0]["name"])].append(key)
    result: dict[str, str] = {}
    for base, keys in sorted(bases.items()):
        for key in sorted(keys):
            if len(keys) == 1:
                result[key] = base
            else:
                suffix = hashlib.sha256(key.encode("utf-8")).hexdigest()[:8]
                result[key] = f"{base}-{suffix}"
    return result


def source_description(package: Json, checksum: str | None) -> str:
    source = package.get("source") or ""
    if source.startswith("git+"):
        repository, _original_ref, revision = git_source_details(source)
        return f"{repository}@{revision or 'unresolved'}"
    checksum_text = f", sha256 {checksum}" if checksum else ""
    return f"crates.io {package['name']} {package['version']}{checksum_text}"


def topological_order(
    package_ids: set[str], dependency_edges: dict[str, set[str]], package_map: dict[str, Json]
) -> tuple[list[str], list[list[str]]]:
    remaining = {
        package_id: set(dependency_edges.get(package_id, set())) & package_ids
        for package_id in package_ids
    }
    order: list[str] = []
    cycles: list[list[str]] = []
    while remaining:
        ready = sorted(
            (package_id for package_id, dependencies in remaining.items() if not dependencies),
            key=lambda package_id: (
                package_map[package_id]["name"],
                package_map[package_id]["version"],
                package_id,
            ),
        )
        if not ready:
            cycle = sorted(
                remaining,
                key=lambda package_id: (
                    package_map[package_id]["name"],
                    package_map[package_id]["version"],
                    package_id,
                ),
            )
            cycles.append(cycle)
            order.extend(cycle)
            break
        for package_id in ready:
            order.append(package_id)
            del remaining[package_id]
        ready_set = set(ready)
        for dependencies in remaining.values():
            dependencies.difference_update(ready_set)
    return order, cycles


def markdown_escape(value: object) -> str:
    if value is None:
        return ""
    return str(value).replace("|", "\\|").replace("\n", " ")


def render_markdown(inventory: Json) -> str:
    packages_by_id = {package["id"]: package for package in inventory["packages"]}
    lines = [
        "# Bevy-coupled dependency inventory",
        "",
        "This file is generated by `tools/bevy_coupled_inventory.py`. Do not edit it by hand.",
        "",
        f"- Classified packages: {len(inventory['packages'])}",
        f"- Upstream source groups: {len(inventory['upstreams'])}",
        f"- Supplemental verification-only upstreams: {len(inventory.get('supplemental_upstreams', []))}",
        f"- Target platform: `{inventory['target_platform']}`",
        f"- Jackdaw lockfile SHA-256: `{inventory['lockfile_sha256']}`",
        "",
        "Official Bevy package identity is derived from the checked-out Bevy workspace; third-party crates whose names begin with `bevy_` remain eligible for this closure.",
        "",
        "## Topological migration order",
        "",
    ]
    for index, package_id in enumerate(inventory["migration_order"], start=1):
        package = packages_by_id[package_id]
        lines.append(
            f"{index}. `{package['name']} {package['version']}` — `{package['upstream_slug']}`"
        )
    lines.extend(
        [
            "",
            "## Classified packages",
            "",
            "| Package | Original source | Original version/rev | Why coupled | Activated by | Local source path | Migration status |",
            "|---|---|---|---|---|---|---|",
        ]
    )
    for package_id in inventory["migration_order"]:
        package = packages_by_id[package_id]
        coupling_names = sorted(
            {item["package"] for item in package["declared_bevy_dependencies"]}
        )
        lines.append(
            "| "
            + " | ".join(
                markdown_escape(value)
                for value in (
                    f"{package['name']} {package['version']}",
                    package["source_description"],
                    package.get("original_revision") or package["version"],
                    ", ".join(coupling_names),
                    ", ".join(package["activated_by"]),
                    package.get("local_source_path") or f"vendor/bevy-coupled/{package['upstream_slug']} (pending mapping)",
                    package["migration_status"],
                )
            )
            + " |"
        )
    lines.extend(["", "## Excluded candidates", ""])
    if not inventory["excluded_candidates"]:
        lines.append("No Bevy-named dependency candidates were excluded.")
    else:
        lines.extend(
            [
                "| Package | Source | Reason |",
                "|---|---|---|",
            ]
        )
        for package in inventory["excluded_candidates"]:
            lines.append(
                f"| {markdown_escape(package['name'] + ' ' + package['version'])} | "
                f"{markdown_escape(package['source'])} | {markdown_escape(package['reason'])} |"
            )
    lines.extend(["", "## Supplemental verification-only upstreams", ""])
    supplemental = inventory.get("supplemental_upstreams", [])
    if not supplemental:
        lines.append("No supplemental upstreams were imported.")
    else:
        lines.extend(
            [
                "These sources are outside the supported Jackdaw closure, but are required to keep a vendored upstream's own mandated verification graph on local Bevy.",
                "",
                "| Upstream | Packages | Original source | Reason | Local source path | Migration status |",
                "|---|---|---|---|---|---|",
            ]
        )
        for upstream in supplemental:
            lines.append(
                "| "
                + " | ".join(
                    markdown_escape(value)
                    for value in (
                        upstream["slug"],
                        ", ".join(upstream["packages"]),
                        upstream["original_source"],
                        upstream["reason"],
                        upstream["local_path"],
                        upstream["migration_status"],
                    )
                )
                + " |"
            )
    if inventory["cycles"]:
        lines.extend(["", "## Migration-order cycles", ""])
        for cycle in inventory["cycles"]:
            lines.append("- " + ", ".join(f"`{packages_by_id[item]['name']}`" for item in cycle))
    lines.append("")
    return "\n".join(lines)


def write_atomic(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_text(content, encoding="utf-8", newline="\n")
    os.replace(temporary, path)


def apply_provenance(inventory: Json, provenance_path: Path) -> None:
    with provenance_path.open("rb") as source:
        provenance = tomllib.load(source)
    records = provenance.get("upstream", [])
    all_by_slug = {record["slug"]: record for record in records}
    if len(all_by_slug) != len(records):
        raise ValueError("provenance contains duplicate upstream slugs")
    classified_records = [record for record in records if not record.get("supplemental", False)]
    supplemental_records = [record for record in records if record.get("supplemental", False)]
    by_slug = {record["slug"]: record for record in classified_records}
    inventory_slugs = {record["slug"] for record in inventory["upstreams"]}
    if len(by_slug) != len(classified_records) or set(by_slug) != inventory_slugs:
        raise ValueError("provenance upstream slugs do not exactly match the inventory")

    manifest_by_slug: dict[str, dict[str, str]] = {}
    for slug, record in by_slug.items():
        mappings: dict[str, str] = {}
        for mapping in record.get("package_paths", []):
            name, separator, manifest = mapping.partition("=")
            if not separator or not name or not manifest or name in mappings:
                raise ValueError(f"invalid or duplicate package mapping for {slug}: {mapping}")
            mappings[name] = manifest.replace("\\", "/")
        manifest_by_slug[slug] = mappings

    packages_by_slug: dict[str, set[str]] = defaultdict(set)
    for package in inventory["packages"]:
        packages_by_slug[package["upstream_slug"]].add(package["name"])
    for slug, package_names in packages_by_slug.items():
        if set(manifest_by_slug[slug]) != package_names:
            raise ValueError(f"provenance package mappings do not match inventory group {slug}")

    for upstream in inventory["upstreams"]:
        record = by_slug[upstream["slug"]]
        upstream["repository"] = record["repository"]
        upstream["original_ref"] = record["original_ref"]
        upstream["original_revision"] = record["original_rev"] or None
        upstream["import_method"] = record["import_method"]
        upstream["local_path"] = record["local_path"]
        upstream["migration_status"] = record["migration_status"]
        upstream["license_files"] = sorted(record["license_files"])
        upstream["local_modifications"] = list(record["local_modifications"])

    for package in inventory["packages"]:
        record = by_slug[package["upstream_slug"]]
        manifest = manifest_by_slug[package["upstream_slug"]][package["name"]]
        package["original_revision"] = record["original_rev"] or package["original_revision"]
        package["import_method"] = record["import_method"]
        package["local_source_path"] = f"{record['local_path'].rstrip('/')}/{manifest}"
        package["migration_status"] = record["migration_status"]

    supplemental_upstreams = []
    for record in sorted(supplemental_records, key=lambda item: item["slug"]):
        packages = []
        for mapping in record.get("package_paths", []):
            name, separator, manifest = mapping.partition("=")
            if not separator or not name or not manifest or name in packages:
                raise ValueError(
                    f"invalid or duplicate supplemental package mapping for {record['slug']}: {mapping}"
                )
            packages.append(name)
        if not packages:
            raise ValueError(f"supplemental upstream {record['slug']} has no package mappings")
        revision = record.get("original_rev") or ""
        registry_versions = record.get("registry_package_versions", [])
        original_source = (
            f"{record['repository']} @ {revision}"
            if revision
            else ", ".join(registry_versions)
        )
        supplemental_upstreams.append(
            {
                "slug": record["slug"],
                "packages": sorted(packages),
                "original_source": original_source,
                "repository": record["repository"],
                "original_ref": record["original_ref"],
                "original_revision": revision or None,
                "registry_package_versions": registry_versions,
                "registry_checksums": record.get("registry_checksums", []),
                "import_method": record["import_method"],
                "local_path": record["local_path"],
                "license_files": sorted(record["license_files"]),
                "bevy_before": record["bevy_before"],
                "bevy_after": record["bevy_after"],
                "migration_status": record["migration_status"],
                "reason": record["supplemental_reason"],
                "local_modifications": list(record["local_modifications"]),
            }
        )
    inventory["supplemental_upstreams"] = supplemental_upstreams

    inventory["provenance_sha256"] = sha256(provenance_path)


def build_inventory(args: argparse.Namespace) -> Json:
    bevy_root = args.bevy_root.resolve()
    bevy_metadata = load_json(args.bevy_metadata)
    official_names = official_bevy_names(bevy_metadata, bevy_root)
    lock_index, lockfile_digest = load_lock_index(args.lockfile)

    matrix_records = []
    combined_packages: dict[str, Json] = {}
    activated_by: dict[str, set[str]] = defaultdict(set)
    active_couplings: dict[str, dict[str, Json]] = defaultdict(dict)
    dependency_edges: dict[str, set[str]] = defaultdict(set)
    all_external_declared_candidates: dict[str, Json] = {}

    for spec in args.matrix:
        metadata = load_json(spec.path)
        packages = {package["id"]: package for package in metadata.get("packages", [])}
        nodes = {node["id"]: node for node in metadata["resolve"].get("nodes", [])}
        roots = resolved_roots(metadata, spec.root_mode)
        reached = reachable_package_ids(metadata, roots)

        matrix_records.append(
            {
                "label": spec.label,
                "root_mode": spec.root_mode,
                "metadata_sha256": sha256(spec.path),
                "reachable_packages": len(reached),
            }
        )

        for package_id, package in packages.items():
            if (
                package.get("source")
                and package["name"] not in official_names
                and declared_couplings(package)
            ):
                all_external_declared_candidates[package_id] = package

        for package_id in reached:
            package = packages[package_id]
            combined_packages[package_id] = package
            if not package.get("source") or package["name"] in official_names:
                continue
            node = nodes[package_id]
            coupled_edges = []
            for edge in node.get("deps", []):
                dependency = packages[edge["pkg"]]
                if is_bevy_named(dependency["name"]):
                    detail = {
                        "package": dependency["name"],
                        "package_id": dependency["id"],
                        "edge_name": edge["name"],
                        "kinds": dependency_kinds(edge),
                    }
                    coupled_edges.append(detail)
                    active_couplings[package_id][
                        json.dumps(detail, sort_keys=True, separators=(",", ":"))
                    ] = detail
            if coupled_edges:
                activated_by[package_id].add(spec.label)

        active_candidate_ids = set(activated_by)
        for package_id in reached & active_candidate_ids:
            node = nodes[package_id]
            for edge in node.get("deps", []):
                dependency_edges[package_id].add(edge["pkg"])

    candidate_ids = set(activated_by)
    package_map = {
        package_id: combined_packages.get(package_id)
        or all_external_declared_candidates[package_id]
        for package_id in candidate_ids
    }
    order, cycles = topological_order(candidate_ids, dependency_edges, package_map)

    provisional_groups: dict[str, Json] = {}
    package_group_details: dict[str, tuple[str, str | None, str | None, str]] = {}
    for package_id in candidate_ids:
        package = package_map[package_id]
        key, repository, original_ref, revision, import_method = upstream_key(package)
        package_group_details[package_id] = (key, original_ref, revision, import_method)
        group = provisional_groups.setdefault(
            key,
            {
                "key": key,
                "repository": repository,
                "original_ref": original_ref,
                "original_revision": revision,
                "import_method": import_method,
                "packages": [],
            },
        )
        group["packages"].append(package)

    for group in provisional_groups.values():
        group["packages"].sort(key=lambda package: (package["name"], package["version"]))
    slugs = allocate_slugs(provisional_groups)

    package_records = []
    for package_id in sorted(
        candidate_ids,
        key=lambda item: (package_map[item]["name"], package_map[item]["version"], item),
    ):
        package = package_map[package_id]
        locked = lock_record(package, lock_index)
        group_key, original_ref, revision, import_method = package_group_details[package_id]
        checksum = locked.get("checksum")
        package_records.append(
            {
                "id": package_id,
                "name": package["name"],
                "version": package["version"],
                "source": package.get("source"),
                "source_description": source_description(package, checksum),
                "checksum": checksum,
                "repository": provisional_groups[group_key]["repository"],
                "original_ref": original_ref,
                "original_revision": revision,
                "license": package.get("license"),
                "license_file": package.get("license_file"),
                "upstream_key": group_key,
                "upstream_slug": slugs[group_key],
                "planned_import_method": import_method,
                "declared_bevy_dependencies": declared_couplings(package),
                "active_bevy_dependencies": sorted(
                    active_couplings[package_id].values(),
                    key=lambda item: (item["package"], item["edge_name"]),
                ),
                "activated_by": sorted(activated_by[package_id]),
                "coupled_dependencies": sorted(
                    dependency_edges.get(package_id, set()) & candidate_ids
                ),
                "local_source_path": None,
                "migration_status": "pending import",
            }
        )

    upstream_records = []
    for key, group in sorted(provisional_groups.items(), key=lambda item: slugs[item[0]]):
        registry_versions = sorted(
            f"{package['name']} {package['version']}"
            for package in group["packages"]
            if (package.get("source") or "").startswith("registry+")
        )
        checksums = []
        for package in group["packages"]:
            locked = lock_record(package, lock_index)
            if locked.get("checksum"):
                checksums.append(f"{package['name']} {package['version']} {locked['checksum']}")
        upstream_records.append(
            {
                "key": key,
                "slug": slugs[key],
                "repository": group["repository"],
                "original_ref": group["original_ref"],
                "original_revision": group["original_revision"],
                "planned_import_method": group["import_method"],
                "registry_package_versions": registry_versions,
                "registry_checksums": sorted(checksums),
                "packages": [
                    {"name": package["name"], "version": package["version"], "id": package["id"]}
                    for package in group["packages"]
                ],
                "local_path": f"editor/jackdaw/vendor/bevy-coupled/{slugs[key]}",
            }
        )

    excluded = []
    for package_id, package in sorted(
        all_external_declared_candidates.items(),
        key=lambda item: (item[1]["name"], item[1]["version"], item[0]),
    ):
        if package_id in candidate_ids:
            continue
        excluded.append(
            {
                "id": package_id,
                "name": package["name"],
                "version": package["version"],
                "source": package.get("source"),
                "reason": "not reachable with an active Bevy-named dependency edge in the supported metadata matrices",
            }
        )

    return {
        "schema_version": 1,
        "target_platform": args.target_platform,
        "lockfile_sha256": lockfile_digest,
        "bevy_metadata_sha256": sha256(args.bevy_metadata),
        "official_bevy_packages": sorted(official_names),
        "matrices": sorted(matrix_records, key=lambda item: item["label"]),
        "packages": package_records,
        "upstreams": upstream_records,
        "migration_order": order,
        "cycles": cycles,
        "excluded_candidates": excluded,
    }


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument(
        "--refresh-existing",
        type=Path,
        help="refresh dynamic provenance and lockfile fields in an existing inventory",
    )
    result.add_argument("--bevy-root", type=Path)
    result.add_argument("--bevy-metadata", type=Path)
    result.add_argument("--lockfile", type=Path)
    result.add_argument(
        "--provenance",
        type=Path,
        help="optional PROVENANCE.toml used to resolve imported paths and status",
    )
    result.add_argument(
        "--matrix",
        action="append",
        type=parse_matrix,
        help="resolved Cargo metadata as LABEL:MODE=PATH; MODE is root or workspace",
    )
    result.add_argument("--target-platform")
    result.add_argument("--output-json", type=Path, required=True)
    result.add_argument("--output-markdown", type=Path, required=True)
    return result


def main() -> int:
    args = parser().parse_args()
    if args.refresh_existing:
        if not args.provenance:
            raise ValueError("--refresh-existing requires --provenance")
        inventory = load_json(args.refresh_existing)
        apply_provenance(inventory, args.provenance)
        if args.lockfile:
            inventory["lockfile_sha256"] = sha256(args.lockfile)
    else:
        required = ("bevy_root", "bevy_metadata", "lockfile", "target_platform")
        if any(getattr(args, name) is None for name in required) or not args.matrix:
            raise ValueError(
                "full inventory generation requires --bevy-root, --bevy-metadata, "
                "--lockfile, --matrix, and --target-platform"
            )
        labels = [matrix.label for matrix in args.matrix]
        if len(labels) != len(set(labels)):
            raise ValueError("matrix labels must be unique")
        inventory = build_inventory(args)
        if args.provenance:
            apply_provenance(inventory, args.provenance)
    json_text = json.dumps(inventory, indent=2, sort_keys=True) + "\n"
    write_atomic(args.output_json, json_text)
    write_atomic(args.output_markdown, render_markdown(inventory))
    print(
        f"classified {len(inventory['packages'])} packages from "
        f"{len(inventory['upstreams'])} upstream groups"
    )
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, json.JSONDecodeError, tomllib.TOMLDecodeError) as error:
        print(f"inventory error: {error}", file=sys.stderr)
        sys.exit(2)
