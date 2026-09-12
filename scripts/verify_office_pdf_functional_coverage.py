#!/usr/bin/env python3
"""Check content-sensitive Office controls alongside the frozen main campaign.

Requires the 11 directed main references to exist. Generates only missing
complementary Office exports; --verify-only checks retained evidence without
starting Office. Sources and main goldens are never rewritten.
"""
import argparse
from datetime import datetime
import hashlib
import json
from pathlib import Path
import posixpath
import shutil
import subprocess
import tempfile
import xml.etree.ElementTree as ET
import zipfile

ROOT = Path(__file__).resolve().parent.parent
GOLDENS = ROOT / "corpus_pdf_conv"


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def windows_path(path):
    return subprocess.check_output(["wslpath", "-w", str(path)], text=True).strip()


def page_texts(pdf):
    subprocess.run(["qpdf", "--check", str(pdf)], check=True, capture_output=True)
    text = subprocess.check_output(["pdftotext", "-layout", str(pdf), "-"], text=True)
    assert text.endswith("\f"), f"Missing page delimiters: {pdf}"
    return [" ".join(page.split()) for page in text[:-1].split("\f")]


def hidden_positions(source):
    with zipfile.ZipFile(source) as package:
        presentation = ET.fromstring(package.read("ppt/presentation.xml"))
        relationships = {r.get("Id"): r.get("Target") for r in
                         ET.fromstring(package.read("ppt/_rels/presentation.xml.rels"))}
        hidden = []
        for index, slide in enumerate(presentation.find("{*}sldIdLst")):
            target = relationships[slide.get("{http://schemas.openxmlformats.org/officeDocument/2006/relationships}id")]
            name = target.lstrip("/") if target.startswith("/") else posixpath.normpath("ppt/" + target)
            if ET.fromstring(package.read(name)).get("show", "1") in ("0", "false"):
                hidden.append(index)
        return hidden


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pwsh", default="pwsh.exe")
    parser.add_argument("--verify-only", action="store_true")
    args = parser.parse_args()
    plan = {a["corpus"] + "/" + a["file"]: a for a in
            map(json.loads, (GOLDENS / "plan.jsonl").read_text().splitlines())}
    records = {p.parent.name + "/" + r["file"]: r for p in GOLDENS.glob("*/manifest.jsonl")
               for r in map(json.loads, p.read_text().splitlines())}
    cases = read_json(ROOT / "crates/ooxmlsdk-pdf-test/data/office-functional-coverage.json")
    host = read_json(GOLDENS / "environment.json")
    for field, script in [("probe_script_sha256", "probe_office_pdf_options.ps1"),
                          ("controller_script_sha256", "invoke_office_pdf_worker.ps1"),
                          ("environment_helper_sha256", "office_pdf_environment.ps1")]:
        assert sha(ROOT / "scripts" / script) == host["environment"][field], script
    store = GOLDENS / "functional-controls"
    store.mkdir(exist_ok=True)
    checks = []

    def control(assignment, options):
        source = assignment["corpus"] + "/" + assignment["file"]
        environment = {key: assignment["requested"][key] for key in
                       ("ui_language", "format_locale", "document_language")}
        environment["time_zone"] = assignment["reference_time_zone"]
        spec = {"schema_version": 2, "file": source, "options": options, "environment": environment}
        identity = {"spec": spec, "source_sha256": assignment["source_sha256"],
                    "host_environment_id": assignment["reference_environment_id"]}
        key = hashlib.sha256(json.dumps(identity, sort_keys=True).encode()).hexdigest()
        destination = store / key
        pdf = destination / "reference.pdf"
        receipt_path = destination / "receipt.json"
        if not receipt_path.exists():
            if args.verify_only:
                raise RuntimeError(f"Missing retained control: {source}, {key}")
            work = Path(tempfile.mkdtemp(prefix="ooxmlsdk-functional-", dir="/tmp"))
            (work / "output").mkdir()
            (work / "plan.jsonl").write_text(json.dumps(spec) + "\n")
            command = [args.pwsh, "-NoProfile", "-NonInteractive", "-STA", "-ExecutionPolicy", "Bypass",
                       "-File", windows_path(ROOT / "scripts/invoke_office_pdf_worker.ps1"),
                       "-CorpusRoot", windows_path(ROOT / "corpus"), "-PlanFile", windows_path(work / "plan.jsonl"),
                       "-OutputRoot", windows_path(work / "output"), "-ProcessIdFile", windows_path(work / "office.pid"),
                       "-TimeoutSeconds", "120"]
            with (work / "controller.log").open("w") as log:
                subprocess.run(command, check=True, stdout=log, stderr=subprocess.STDOUT)
            assert read_json(work / "environment-restored.json")["restored"] is True
            destination.mkdir(exist_ok=True)
            shutil.copyfile(work / "output/case-000.pdf", pdf)
            shutil.copyfile(work / "output/case-000.json", receipt_path)
            (destination / "identity.json").write_text(json.dumps(identity, indent=2) + "\n")
            shutil.copyfile(work / "environment-restored.json", destination / "environment-restored.json")
        assert read_json(destination / "identity.json") == identity
        receipt = read_json(receipt_path)
        assert receipt["schema_version"] == 2 and receipt["file"] == source
        assert receipt["source_sha256"] == assignment["source_sha256"] == sha(ROOT / "corpus" / source)
        assert receipt["options"] == options and receipt["output_sha256"] == sha(pdf)
        assert receipt["output_bytes"] == pdf.stat().st_size
        assert read_json(destination / "environment-restored.json")["restored"] is True
        assert all(receipt["environment"][key] == value for key, value in environment.items())
        clock = receipt["environment"]["clock"]
        times = [datetime.fromisoformat(clock[key]) for key in ("opened_at_utc", "refresh_started_at_utc",
                 "refresh_completed_at_utc", "export_started_at_utc", "export_completed_at_utc")]
        assert times == sorted(times)
        return pdf

    for case in cases:
        source = case["source"]
        assignment, record = plan[source], records[source]
        assert assignment["source_sha256"] == case["source_sha256"] == sha(ROOT / "corpus" / source)
        assert assignment["reference_environment_id"] == record["environment_id"] == host["environment_id"]
        assert record["status"] == "converted" and record["configuration_id"] == assignment["configuration_id"]
        golden = GOLDENS / assignment["corpus"] / record["output"]
        assert sha(golden) == record["output_sha256"]
        if case["feature"].startswith("hidden-slides-"):
            hidden = hidden_positions(ROOT / "corpus" / source)
            assert hidden, source
            for quality in ("print", "screen"):
                versions = {}
                for include in (False, True):
                    options = dict(assignment["office"], quality=quality, print_hidden_slides=include,
                                   page_from=None, page_to=None)
                    pdf = golden if options == assignment["office"] else control(assignment, options)
                    versions[include] = page_texts(pdf)
                    assert len(versions[include]) == case["expected_source_pages"] - (0 if include else len(hidden)), source
                visible = [text for index, text in enumerate(versions[True]) if index not in hidden]
                assert versions[False] == visible, f"Hidden-slide membership differs: {source}, {quality}"
                checks.append({"source": source, "feature": "hidden-slide-membership", "quality": quality,
                               "with_hidden": len(versions[True]), "without_hidden": len(versions[False])})
        else:
            options = dict(assignment["office"], page_from=None, page_to=None)
            full = page_texts(control(assignment, options))
            selected = page_texts(golden)
            assert len(full) == case["expected_source_pages"], f"Source pagination changed: {source}: {len(full)}"
            first, last = assignment["office"]["page_from"], assignment["office"]["page_to"]
            assert selected == full[first - 1:last], f"Wrong selected-page content: {source}"
            assert selected != full[:len(selected)], f"Fixture cannot distinguish first-page substitution: {source}"
            checks.append({"source": source, "feature": case["feature"], "from": first, "to": last,
                           "full_pages": len(full), "selected_pages": len(selected)})
        print(f"PASS: {source}: {case['feature']}", flush=True)
    report = {"schema_version": 1, "environment_id": host["environment_id"],
              "coverage_manifest_sha256": sha(ROOT / "crates/ooxmlsdk-pdf-test/data/office-functional-coverage.json"),
              "checks": checks}
    (store / "verification.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"PASS: {len(checks)} content-sensitive checks; {store}")


if __name__ == "__main__":
    main()
