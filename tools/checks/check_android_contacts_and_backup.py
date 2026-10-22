#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/checks/check_android_contacts_and_backup.py
#
# >>> help
# Does the Android client reach no address book, and leave the phone
# through no platform backup or transfer?
#
#   tools/checks/check_android_contacts_and_backup.py [--root <dir>]
#
# THE RULES, quoted:
#
#   ADR-0059 decision 6: "On Android the client declares no contacts
#   permission (`READ_CONTACTS`, `WRITE_CONTACTS`, `GET_ACCOUNTS`), uses
#   no contacts provider or picker, and exports nothing to one ... A tree
#   check over the Android manifest and the Gradle dependency list pins
#   the first half".
#
#   ADR-0060 decision 8: "on Android the application declares
#   `android:allowBackup="false"` and extraction rules that exclude every
#   domain from both `cloud-backup` and `device-transfer` ... and never
#   adopts a backup agent, a platform account sync or a device-to-device
#   transfer path"; the manifest check plan section 29 step 2 owes "pins
#   these two declarations as well".
#
# WHAT IS READ, under apps/human-android/:
#   - every AndroidManifest.xml of a source set that can reach an APK the
#     person installs (main, a build type, a flavour), parsed as XML; the
#     androidTest and test source sets are test tooling and not read;
#   - the main manifest's <application>: allowBackup must be "false",
#     dataExtractionRules must be set, and backupAgent, fullBackupContent
#     and fullBackupOnly must be absent -- in every manifest read, since a
#     variant manifest merges over main;
#   - android:dataExtractionRules set in the main manifest alone: a
#     variant manifest setting it, which would merge over main's, fails;
#   - the extraction-rules file the main manifest names, in main and in
#     every other non-test source set that has one (a variant's res/xml
#     replaces main's at the resource merge), in res/xml and in every
#     qualified res/xml-* directory (xml-v31 is the copy the platform
#     picks where the rules are read): <cloud-backup> and
#     <device-transfer> each present, each excluding every domain at the
#     path ".", and no <include> anywhere;
#   - permissions: no <uses-permission>, <uses-permission-sdk-23> or
#     <permission> naming one of the three, in any manifest read;
#   - the contacts provider: no manifest, Kotlin, Java, Gradle script,
#     version catalog or Rust source of the Android client (the app, its
#     Rust library, crates/human/android-platform) naming
#     `ContactsContract`, `android.provider.Contacts`, the provider's
#     authority `com.android.contacts` or the legacy `content://contacts`
#     -- read raw, so a comment naming
#     one fails too, as the picker and the provider are reached by those
#     names and no other;
#   - the Gradle dependency list: no line of a gradle.lockfile, a Gradle
#     script or a version catalog naming a coordinate whose group or
#     artifact contains "contact" (any case).
#
# FAIL-CLOSED: an absent main manifest, an absent extraction-rules file,
# a manifest that does not parse, or no lockfile read is a failure, not
# a pass. What the check does NOT see is the desktop half of decision 6
# (the client reads no system address book there), which no tree check
# can pin: the desktop has no manifest.
#
# Exit codes:
#   0  every rule holds
#   1  a rule does not hold, or an input the rule needs is missing
#   2  invocation error
# <<< help

import pathlib
import re
import sys
import xml.etree.ElementTree as ET

HELP_RE = re.compile(r"^# >>> help$(.*?)^# <<< help$", re.M | re.S)
ANDROID_NS = "http://schemas.android.com/apk/res/android"
A = "{" + ANDROID_NS + "}"

CONTACTS_PERMISSIONS = {
    "android.permission.READ_CONTACTS",
    "android.permission.WRITE_CONTACTS",
    "android.permission.GET_ACCOUNTS",
}
PERMISSION_TAGS = ("uses-permission", "uses-permission-sdk-23", "permission")
FORBIDDEN_APPLICATION_ATTRS = ("backupAgent", "fullBackupContent", "fullBackupOnly")
# Every domain data_extraction_rules.xml accepts, credential- and
# device-protected: a domain left out is a directory a store or a key
# file could be moved into and then copied off the phone.
EXTRACTION_DOMAINS = {
    "root", "file", "database", "sharedpref", "external",
    "device_root", "device_file", "device_database", "device_sharedpref",
}
EXTRACTION_SECTIONS = ("cloud-backup", "device-transfer")
PROVIDER_NAMES = (
    "ContactsContract",
    "android.provider.Contacts",
    "com.android.contacts",
    "content://contacts",
)
TEST_SOURCE_SETS = {"androidTest", "test"}
SKIP_DIRS = {"build", ".gradle", ".cxx", "target", ".git"}

problems: list[str] = []


def report(msg: str) -> None:
    problems.append(msg)
    print(f"  ✗ {msg}", file=sys.stderr)


def walk(base: pathlib.Path):
    if not base.is_dir():
        return
    for path in sorted(base.rglob("*")):
        rel = path.relative_to(base).parts
        if any(p in SKIP_DIRS for p in rel):
            continue
        if path.is_file():
            yield path


def source_set(manifest: pathlib.Path) -> str:
    # .../src/<set>/AndroidManifest.xml
    return manifest.parent.name


def parse(path: pathlib.Path, root: pathlib.Path):
    try:
        return ET.parse(path).getroot()
    except (ET.ParseError, OSError) as e:
        report(f"{path.relative_to(root)}: does not parse as XML ({e})")
        return None


def check_manifest(path: pathlib.Path, root: pathlib.Path, main: bool):
    rel = path.relative_to(root)
    doc = parse(path, root)
    if doc is None:
        return None
    for tag in PERMISSION_TAGS:
        for el in doc.iter(tag):
            name = el.get(A + "name", "")
            if name in CONTACTS_PERMISSIONS:
                report(f"{rel}: <{tag}> declares {name} (ADR-0059 decision 6)")
    apps = list(doc.iter("application"))
    for app in apps:
        for attr in FORBIDDEN_APPLICATION_ATTRS:
            if app.get(A + attr) is not None:
                report(f"{rel}: <application> sets android:{attr} (ADR-0060 decision 8)")
        backup = app.get(A + "allowBackup")
        if backup is not None and backup != "false":
            report(f'{rel}: <application> sets android:allowBackup="{backup}", not "false" (ADR-0060 decision 8)')
    if not main:
        for app in apps:
            if app.get(A + "dataExtractionRules") is not None:
                report(f"{rel}: <application> sets android:dataExtractionRules in a variant manifest, over main's (ADR-0060 decision 8)")
        return None
    if len(apps) != 1:
        report(f"{rel}: the main manifest has {len(apps)} <application> elements, not one")
        return None
    app = apps[0]
    if app.get(A + "allowBackup") != "false":
        report(f'{rel}: <application> does not set android:allowBackup="false" (ADR-0060 decision 8)')
    rules = app.get(A + "dataExtractionRules")
    if rules is None:
        report(f"{rel}: <application> sets no android:dataExtractionRules (ADR-0060 decision 8)")
        return None
    m = re.fullmatch(r"@xml/([A-Za-z0-9_]+)", rules)
    if not m:
        report(f"{rel}: android:dataExtractionRules is '{rules}', not an @xml/ resource")
        return None
    return f"{m.group(1)}.xml"


def check_extraction_rules(path: pathlib.Path, root: pathlib.Path, required: bool) -> None:
    rel = path.relative_to(root)
    if not path.is_file():
        if required:
            report(f"{rel}: the extraction-rules file the main manifest names is missing")
        return
    doc = parse(path, root)
    if doc is None:
        return
    if doc.tag != "data-extraction-rules":
        report(f"{rel}: the root element is <{doc.tag}>, not <data-extraction-rules>")
    for el in doc.iter("include"):
        report(f"{rel}: an <include domain=\"{el.get('domain')}\"> re-admits data (ADR-0060 decision 8)")
    for section in EXTRACTION_SECTIONS:
        found = doc.findall(section)
        if len(found) != 1:
            report(f"{rel}: {len(found)} <{section}> sections, not one (ADR-0060 decision 8)")
            continue
        excluded = {
            el.get("domain") for el in found[0].findall("exclude") if el.get("path") == "."
        }
        missing = sorted(EXTRACTION_DOMAINS - excluded)
        if missing:
            report(f"{rel}: <{section}> does not exclude the domain(s) {', '.join(missing)} at path \".\" (ADR-0060 decision 8)")


def main(argv: list[str]) -> int:
    root = pathlib.Path(__file__).resolve().parent.parent.parent
    args = argv[1:]
    while args:
        a = args.pop(0)
        if a in ("-h", "--help"):
            m = HELP_RE.search(pathlib.Path(__file__).read_text(encoding="utf-8"))
            print("\n".join(line[2:] if line.startswith("# ") else line[1:] for line in m.group(1).strip("\n").splitlines()))
            return 0
        if a == "--root":
            if not args:
                print("check_android_contacts_and_backup: --root needs a directory", file=sys.stderr)
                return 2
            root = pathlib.Path(args.pop(0)).resolve()
        else:
            print(f"check_android_contacts_and_backup: unexpected argument: {a}", file=sys.stderr)
            return 2

    app = root / "apps" / "human-android"
    android = app / "android"
    main_manifest = android / "app" / "src" / "main" / "AndroidManifest.xml"
    if not main_manifest.is_file():
        report(f"{main_manifest.relative_to(root)}: missing; an absent manifest is not a clean one")
        return finish()

    manifests = [
        p for p in walk(android)
        if p.name == "AndroidManifest.xml" and source_set(p) not in TEST_SOURCE_SETS
    ]
    rules = None
    for m in manifests:
        r = check_manifest(m, root, m == main_manifest)
        if m == main_manifest:
            rules = r
    if rules is not None:
        # Main's copy must exist; any other shipped source set's copy
        # replaces it at the resource merge, so each one present is read.
        sets = android / "app" / "src"
        for d in sorted(p for p in sets.iterdir() if p.is_dir()):
            if d.name in TEST_SOURCE_SETS:
                continue
            # Every res/xml* directory: a qualified one (xml-v31) is the
            # copy the platform picks on the devices that read the rules.
            res = d / "res"
            dirs = sorted(x for x in res.iterdir() if x.is_dir() and (x.name == "xml" or x.name.startswith("xml-"))) if res.is_dir() else []
            if d.name == "main" and not any(x.name == "xml" for x in dirs):
                dirs.insert(0, res / "xml")
            for x in dirs:
                check_extraction_rules(x / rules, root, d.name == "main" and x.name == "xml")

    # Raw scans: a provider name anywhere the Android client is built from,
    # and a contacts coordinate anywhere the Gradle dependency list is.
    scanned = 0
    lockfiles = 0
    raw_roots = [android, app / "src", root / "crates" / "human" / "android-platform"]
    for base in raw_roots:
        for path in walk(base):
            parts = set(path.relative_to(base).parts)
            if parts & TEST_SOURCE_SETS:
                continue
            if path.suffix not in {".xml", ".kt", ".java", ".kts", ".gradle", ".toml", ".rs", ".lockfile"} and path.name != "gradle.lockfile":
                continue
            try:
                text = path.read_text(encoding="utf-8")
            except (UnicodeDecodeError, OSError):
                continue
            scanned += 1
            rel = path.relative_to(root)
            for i, line in enumerate(text.splitlines(), 1):
                for name in PROVIDER_NAMES:
                    if name in line:
                        report(f"{rel}:{i}: names {name}, the contacts provider (ADR-0059 decision 6)")
            if path.name.endswith(".lockfile") or path.suffix in {".kts", ".gradle", ".toml"}:
                if path.name.endswith(".lockfile"):
                    lockfiles += 1
                for i, line in enumerate(text.splitlines(), 1):
                    coord = line.split("=", 1)[0] if path.name.endswith(".lockfile") else line
                    if re.search(r"[\"']?[\w.-]*contact[\w.-]*:[\w.-]+", coord, re.I):
                        report(f"{rel}:{i}: a dependency coordinate names a contacts library (ADR-0059 decision 6)")
    if lockfiles == 0:
        report(f"{android.relative_to(root)}: no Gradle lockfile read; the dependency list was not checked")
    return finish(len(manifests), scanned)


def finish(manifests: int = 0, scanned: int = 0) -> int:
    if problems:
        print(f"\ncheck_android_contacts_and_backup: {len(problems)} problem(s).", file=sys.stderr)
        return 1
    print(
        f"check_android_contacts_and_backup: OK — {manifests} manifest(s), the extraction rules "
        f"and {scanned} source file(s) read: no contacts permission, provider or library; "
        f"allowBackup false; every domain excluded from cloud-backup and device-transfer."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
