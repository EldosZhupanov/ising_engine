#!/usr/bin/env python3
"""Post-hoc diagnosis of the frozen GPT entry's trajectory/log mismatch."""

import hashlib
import json
import urllib.request
import xml.etree.ElementTree as ET

from replay import BASE, CASES, fetch


CODEX = "20260219_mini-v2.0.0_gpt-5-2-codex"
HIGH = "20260217_mini-v2.0.0_gpt-5-2-high"
LISTING_HASH = {
    CODEX: "0246259a25da122721290a23a65ce6822fa2a4da459703017920e7471cbb7ad1",
    HIGH: "dbddef4607f3e23428073b3ed3f66e4a2a5275927c8eb0ca21f7ae4ed31b31cc",
}


def listing(entry):
    url = (
        "https://swe-bench-submissions.s3.amazonaws.com/"
        f"?list-type=2&prefix=bash-only/{entry}/trajs/&max-keys=1000"
    )
    with urllib.request.urlopen(url, timeout=30) as response:
        raw = response.read()
    digest = hashlib.sha256(raw).hexdigest()
    if digest != LISTING_HASH[entry]:
        raise ValueError(f"listing changed for {entry}: {digest}")
    objects = {}
    for item in ET.fromstring(raw):
        if not item.tag.endswith("Contents"):
            continue
        fields = {x.tag.rsplit("}", 1)[-1]: x.text for x in item}
        relative = fields["Key"].split("/trajs/", 1)[1]
        if relative.endswith(".traj.json"):
            objects[relative] = [fields["ETag"], int(fields["Size"])]
    return objects, url, digest


def main():
    a, a_url, a_sha = listing(CODEX)
    b, b_url, b_sha = listing(HIGH)
    if len(a) != 500 or a != b:
        raise ValueError("trajectory listing identities differ")
    rows = []
    for instance, hashes in CASES[CODEX].items():
        path = f"trajs/{instance}/{instance}.traj.json"
        codex_raw, codex_url, codex_sha = fetch(CODEX, path, hashes[0])
        high_raw, high_url, high_sha = fetch(HIGH, path, hashes[0])
        if codex_raw != high_raw:
            raise ValueError(f"trajectory contents differ for {instance}")
        trajectory = json.loads(codex_raw)
        submission = trajectory["info"]["submission"]
        codex_log_raw, codex_log_url, codex_log_sha = fetch(
            CODEX, f"logs/{instance}/patch.diff", hashes[1]
        )
        high_log_url = BASE + HIGH + f"/logs/{instance}/patch.diff"
        with urllib.request.urlopen(high_log_url, timeout=30) as response:
            high_log_raw = response.read()
        if high_log_raw.decode() != submission or codex_log_raw.decode() == submission:
            raise ValueError(f"patch association differs from observed finding: {instance}")
        rows.append(
            {
                "instance_id": instance,
                "codex_trajectory_url": codex_url,
                "high_trajectory_url": high_url,
                "trajectory_sha256": codex_sha,
                "trajectory_bytes_identical": codex_raw == high_raw,
                "trajectory_model_name": trajectory["info"]["config"]["model"]["model_name"],
                "codex_log_patch_url": codex_log_url,
                "codex_log_patch_sha256": codex_log_sha,
                "high_log_patch_url": high_log_url,
                "high_log_patch_sha256": hashlib.sha256(high_log_raw).hexdigest(),
                "submission_equals_high_log_patch": True,
                "submission_equals_codex_log_patch": False,
            }
        )
    print(
        json.dumps(
            {
                "code_entry": CODEX,
                "high_entry": HIGH,
                "listing_urls": [a_url, b_url],
                "listing_sha256": [a_sha, b_sha],
                "matched_traj_keys_with_same_etag_and_size": len(a),
                "cases": rows,
            },
            indent=2,
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
