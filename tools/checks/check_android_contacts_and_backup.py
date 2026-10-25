#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/checks/check_android_contacts_and_backup.py
#
# >>> help
# Does the Android client reach no address book, leave the phone through
# no platform backup or transfer, ask no permission its records do not
# name, and use the camera only to scan a code?
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
#   Plan section 29 step 2 (A 2026-10-22, ADR-0062 decision 8): "it pins
#   the app's permission set as an allow-list: exactly `INTERNET`,
#   `ACCESS_NETWORK_STATE`, `FOREGROUND_SERVICE`,
#   `FOREGROUND_SERVICE_REMOTE_MESSAGING`, `POST_NOTIFICATIONS`,
#   `USE_BIOMETRIC` and, from step 5, `CAMERA` with `<uses-feature
#   android:name="android.hardware.camera" android:required="false"/>`
#   beside it -- any other permission or a required camera feature fails
#   the check by name ... and from step 5 it holds the camera APIs to the
#   Scan module alone."
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
#   - the permission allow-list: every <uses-permission> and
#     <uses-permission-sdk-23> in any manifest read names one of the
#     seven above, each named by the main manifest, and no manifest
#     defines a <permission> of its own;
#   - the camera feature: the main manifest declares
#     android.hardware.camera with android:required="false" -- the
#     CAMERA permission alone makes the platform's store take that
#     feature as required -- and every <uses-feature> naming a camera
#     (android.hardware.camera*) in any manifest read says
#     android:required="false" in so many words, the attribute's absence
#     meaning "true";
#   - the camera APIs (camera2, the legacy Camera, CameraX, the NDK's
#     ACamera*, the camera service, the capture intents, the CAMERA
#     permission and the camera features by name) in the Kotlin, Java,
#     Rust, Gradle and resource sources of apps/human-android, read raw:
#     anywhere in the Scan module, CodeCamera.kt; in MainActivity.kt only
#     its use of the module -- the CAMERA permission asked and the
#     camera feature asked of the package manager -- and nowhere else
#     (the manifests' declarations are the allow-list's);
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
# a manifest that does not parse, no lockfile read, or no Scan module
# found is a failure, not a pass. What the check does NOT see: the
# desktop half of decision 6 (the client reads no system address book
# there), which no tree check can pin, the desktop having no manifest;
# and a permission a library's own manifest merges into the APK, which
# only the merged manifest, a build output, holds -- the lockfile names
# no library that declares one today, and that is read, not checked.
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
USES_PERMISSION_TAGS = ("uses-permission", "uses-permission-sdk-23")
# Plan section 29 step 2's list (A 2026-10-22): a permission enters the
# app only through a record that names it there, and then here.
ALLOWED_PERMISSIONS = {
    "android.permission." + name
    for name in (
        "INTERNET",
        "ACCESS_NETWORK_STATE",
        "FOREGROUND_SERVICE",
        "FOREGROUND_SERVICE_REMOTE_MESSAGING",
        "POST_NOTIFICATIONS",
        "USE_BIOMETRIC",
        "CAMERA",
    )
}
CAMERA_FEATURE = "android.hardware.camera"
# The camera APIs by the names they are reached by. Read raw over the
# Android client's sources, so a comment naming one counts too.
CAMERA_API = re.compile(
    r"android\.hardware\.camera2|android/hardware/camera2"
    r"|android\.hardware\.Camera\b|android/hardware/Camera\b"
    r"|androidx\.camera|\bCameraManager\b|\bCAMERA_SERVICE\b"
    r"|\bACamera\w*|\bACTION_IMAGE_CAPTURE\w*|\bACTION_VIDEO_CAPTURE\b"
    r"|\bINTENT_ACTION_STILL_IMAGE_CAMERA\w*|\bINTENT_ACTION_VIDEO_CAMERA\b"
    r"|permission\.CAMERA\b|\bFEATURE_CAMERA\w*|android\.hardware\.camera\b[\w.]*"
)
SCAN_MODULE = "CodeCamera.kt"
# MainActivity's use of the Scan module, and nothing else of the camera.
SCAN_CALLER = "MainActivity.kt"
SCAN_CALLER_USES = {"permission.CAMERA", "FEATURE_CAMERA_ANY"}
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
    declared = set()
    for tag in USES_PERMISSION_TAGS:
        for el in doc.iter(tag):
            name = el.get(A + "name", "")
            declared.add(name)
            if name not in ALLOWED_PERMISSIONS and name not in CONTACTS_PERMISSIONS:
                report(f"{rel}: <{tag}> declares {name}, which the permission allow-list does not name (plan section 29 step 2)")
    for el in doc.iter("permission"):
        report(f"{rel}: <permission> defines {el.get(A + 'name', '')}, and the app defines none (plan section 29 step 2)")
    camera_declared = False
    for el in doc.iter("uses-feature"):
        name = el.get(A + "name", "")
        if not name.startswith(CAMERA_FEATURE):
            continue
        required = el.get(A + "required")
        if required != "false":
            said = "no android:required, which means true" if required is None else f'android:required="{required}"'
            report(f"{rel}: <uses-feature> {name} is required ({said}); a camera is never required (plan section 29 step 2)")
        if name == CAMERA_FEATURE:
            camera_declared = True
    if main:
        for name in sorted(ALLOWED_PERMISSIONS - declared):
            report(f"{rel}: the main manifest does not declare {name}, which the permission allow-list names (plan section 29 step 2)")
        if not camera_declared:
            report(f'{rel}: no <uses-feature android:name="{CAMERA_FEATURE}" android:required="false"/> beside CAMERA (plan section 29 step 2)')
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


def check_camera_apis(app: pathlib.Path, root: pathlib.Path) -> None:
    """The camera APIs in the Scan module alone (plan section 29 step 2)."""
    modules = 0
    for path in walk(app):
        rel_parts = set(path.relative_to(app).parts)
        if rel_parts & TEST_SOURCE_SETS or path.name == "AndroidManifest.xml":
            continue
        if path.suffix not in {".kt", ".java", ".rs", ".kts", ".gradle", ".toml", ".xml", ".lockfile"}:
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        if path.name == SCAN_MODULE:
            modules += 1
            continue
        rel = path.relative_to(root)
        for i, line in enumerate(text.splitlines(), 1):
            for m in CAMERA_API.finditer(line):
                if path.name == SCAN_CALLER and m.group(0) in SCAN_CALLER_USES:
                    continue
                report(f"{rel}:{i}: names {m.group(0)}, a camera API outside the Scan module {SCAN_MODULE} (plan section 29 step 2)")
    if modules != 1:
        report(f"{app.relative_to(root)}: {modules} Scan module(s) {SCAN_MODULE} found, not one; the camera APIs were not held to it")


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

    check_camera_apis(app, root)

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
        f"allowBackup false; every domain excluded from cloud-backup and device-transfer; "
        f"only the allow-listed permissions, the camera not required, its APIs in {SCAN_MODULE} alone."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
