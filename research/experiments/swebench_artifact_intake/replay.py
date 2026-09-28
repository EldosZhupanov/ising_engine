#!/usr/bin/env python3
"""Replay the frozen six-case SWE-bench public-artifact intake.

This checks artifact identity and patch lineage; it does not grade a patch or
infer a final success claim from an agent's act of submitting it.
"""

import hashlib
import json
import urllib.request


BASE = "https://swe-bench-submissions.s3.amazonaws.com/bash-only/"
CASES = {
    "20260219_mini-v2.0.0_gpt-5-2-codex": {
        "sympy__sympy-13798": (
            "365ab996068aa1ff810a5b9c6d60fce04e50ceccc866b0ce0be0e3c2bf408a80",
            "0a1b75780791d7947bbdf5fd6ff1d910d4a0a026ff2fdd37b48caf06c79cbf61",
            "b683803acfeaf43eb679612ec93b0670f758df035b63adaba8db66af86ef2244",
            "2f54521fc4dafd6e07d895fccaf72985f04e889739c98b7688b95fb3e05bfe50",
            "392c8215d32f93a66b3c61fd70f2c52de03acd75fd92586c067b17408e7569c5",
        ),
        "pytest-dev__pytest-5631": (
            "dba44fdcadb96cf0819905cad74618111ddb02beea672da55b350b04c1ee7377",
            "728595d95457fc01e52c42233556b7bd0bb7f681378248c45656c8656cf38de1",
            "6a93309d3d23e50f12f6549431c2bc79b9b92c97ee89864bda83f1a6887331c6",
            "7d3c4210a5d48313e477ba1797b709639110777eb7ec8d4ce0b9100f345acb8c",
            "138d80b46f77daad8fe7b8f80074577496b0aec38cd6389d7e0ac868fe84739e",
        ),
        "sympy__sympy-17318": (
            "08f659c0567c5572107b67462fd229ebdbdd8890e4d0d954caeb6e2f8f83f5b5",
            "ec660195991c803d89ece67e457d400281458499814ad37f442047d29c570aff",
            "3c81e1c4c39c8be59a0f5c8df422985755d94625bb6d5cff557584168c308a05",
            "fd759f7b23cc1b9dda1d45c531f1fb40f50de8e3fa7768c923e7916cf9b17d61",
            "56adb396d6657acc5a33c9c111492c30ee2da5160386bcfc7d929aa7b6f354af",
        ),
    },
    "20260217_mini-v2.0.0_claude-4-5-haiku-high": {
        "sympy__sympy-13798": ("4e06572bd17e838022ab5303824137c4d2146e8d60129f1162db1cd169a7124f",),
        "pytest-dev__pytest-5631": ("66653633cd490c957e5472e983c4104cda9ca79fa753036c481425367a460e34",),
        "sympy__sympy-17318": ("b7e2055ecb66bb311adca204b38c4dd7d9ca51a34b9b6a1e5e887d0bf078ce89",),
    },
}


def fetch(entry, path, expected):
    url = BASE + entry + "/" + path
    with urllib.request.urlopen(url, timeout=30) as response:
        raw = response.read()
    digest = hashlib.sha256(raw).hexdigest()
    if digest != expected:
        raise ValueError(f"artifact changed: {url}: {digest} != {expected}")
    return raw, url, digest


def main():
    rows = []
    for entry, instances in CASES.items():
        for instance, digests in instances.items():
            raw, trajectory_url, trajectory_sha = fetch(
                entry, f"trajs/{instance}/{instance}.traj.json", digests[0]
            )
            trajectory = json.loads(raw)
            submitted = trajectory["info"]["submission"]
            exits = [m for m in trajectory["messages"] if m.get("role") == "exit"]
            assistant_text = [
                m["content"]
                for m in trajectory["messages"]
                if m.get("role") == "assistant" and isinstance(m.get("content"), str)
                and m["content"].strip()
            ]
            if trajectory["instance_id"] != instance or len(exits) != 1:
                raise ValueError(f"unexpected trajectory identity/exit: {entry}/{instance}")
            row = {
                "entry": entry,
                "instance_id": instance,
                "trajectory_url": trajectory_url,
                "trajectory_sha256": trajectory_sha,
                "exit_status": trajectory["info"]["exit_status"],
                "submitted_patch_sha256": hashlib.sha256(submitted.encode()).hexdigest(),
                "submission_equals_exit_content": submitted == exits[0]["content"],
                "submitted_patch_nonempty": bool(submitted),
                "assistant_text_messages": len(assistant_text),
                "last_assistant_text": assistant_text[-1] if assistant_text else None,
                "report_and_test_output_present": False,
                "official_regrader_sufficiency_verified": False,
            }
            if len(digests) == 5:
                paths = ("patch.diff", "report.json", "test_output.txt", "job-output.json")
                artifacts = {}
                for name, expected in zip(paths, digests[1:]):
                    content, url, sha = fetch(entry, f"logs/{instance}/{name}", expected)
                    artifacts[name] = (content, url, sha)
                log_patch = artifacts["patch.diff"][0].decode()
                report = json.loads(artifacts["report.json"][0])
                job = json.loads(artifacts["job-output.json"][0])
                def changed_lines(patch):
                    return {
                        line for line in patch.splitlines()
                        if line.startswith(("+", "-"))
                        and not line.startswith(("+++", "---"))
                    }

                submitted_lines = changed_lines(submitted)
                log_lines = changed_lines(log_patch)
                row.update(
                    log_patch_url=artifacts["patch.diff"][1],
                    log_patch_sha256=artifacts["patch.diff"][2],
                    report_url=artifacts["report.json"][1],
                    report_sha256=artifacts["report.json"][2],
                    test_output_sha256=artifacts["test_output.txt"][2],
                    job_output_sha256=artifacts["job-output.json"][2],
                    resolved=report["resolved"],
                    report_and_test_output_present=bool(artifacts["test_output.txt"][0]),
                    submission_equals_log_patch=submitted == log_patch,
                    changed_lines_only_in_submission=len(submitted_lines - log_lines),
                    changed_lines_only_in_log=len(log_lines - submitted_lines),
                    log_patch_equals_diff_before=log_patch == job["diff_before"],
                    diff_before_equals_diff_after=job["diff_before"] == job["diff_after"],
                    diff_changed=job["diff_changed"],
                )
            rows.append(row)
    print(json.dumps({"cases": rows}, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
