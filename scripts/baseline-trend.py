#!/usr/bin/env python3
import json
import math
import os
import re
import sys
import urllib.request
import zipfile
from datetime import datetime, timezone
from io import BytesIO


def api_get(url: str, token: str):
    req = urllib.request.Request(
        url,
        headers={
            "Authorization": f"Bearer {token}",
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
            "User-Agent": "symm-baseline-trend",
        },
    )
    with urllib.request.urlopen(req) as resp:
        return json.loads(resp.read().decode("utf-8"))


def download_zip(url: str, token: str) -> bytes:
    req = urllib.request.Request(
        url,
        headers={
            "Authorization": f"Bearer {token}",
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
            "User-Agent": "symm-baseline-trend",
        },
    )
    with urllib.request.urlopen(req) as resp:
        return resp.read()


def percentile(values, p):
    if not values:
        return None
    arr = sorted(values)
    idx = max(0, min(len(arr) - 1, math.ceil(len(arr) * p) - 1))
    return arr[idx]


def parse_perf_log(content: str):
    pattern = re.compile(r"\[symm-perf\].*?event=([a-z_]+)\s+elapsed_ms=(\d+)")
    events = {}
    for line in content.splitlines():
        m = pattern.search(line)
        if not m:
            continue
        event = m.group(1)
        elapsed = int(m.group(2))
        events.setdefault(event, []).append(elapsed)
    return events


def run():
    repo = os.environ.get("GITHUB_REPOSITORY", "")
    token = os.environ.get("GITHUB_TOKEN", "")
    out_dir = os.environ.get("OUT_DIR", "artifacts")
    lookback = int(os.environ.get("LOOKBACK", "7"))
    ls_threshold = int(os.environ.get("LS_THRESHOLD_MS", "1000"))
    show_p95_threshold = int(os.environ.get("SHOW_P95_THRESHOLD_MS", "200"))
    rollback_threshold = int(os.environ.get("ROLLBACK_THRESHOLD_PCT", "100"))

    if not repo or not token:
        raise RuntimeError("GITHUB_REPOSITORY/GITHUB_TOKEN is required")
    owner, name = repo.split("/", 1)
    os.makedirs(out_dir, exist_ok=True)

    perf_runs = api_get(
        f"https://api.github.com/repos/{owner}/{name}/actions/workflows/perf-baseline.yml/runs?status=completed&per_page={lookback}",
        token,
    ).get("workflow_runs", [])
    rollback_runs = api_get(
        f"https://api.github.com/repos/{owner}/{name}/actions/workflows/rollback-metrics.yml/runs?status=completed&per_page={lookback}",
        token,
    ).get("workflow_runs", [])

    perf_report = []
    for run in perf_runs:
        if run.get("conclusion") != "success":
            continue
        artifacts = api_get(
            f"https://api.github.com/repos/{owner}/{name}/actions/runs/{run['id']}/artifacts",
            token,
        ).get("artifacts", [])
        target = next((a for a in artifacts if a["name"].startswith("perf-baseline-")), None)
        if not target:
            continue
        data = download_zip(target["archive_download_url"], token)
        with zipfile.ZipFile(BytesIO(data), "r") as zf:
            file_name = next((n for n in zf.namelist() if n.endswith("perf-baseline.log")), None)
            if not file_name:
                continue
            content = zf.read(file_name).decode("utf-8", errors="replace")
        events = parse_perf_log(content)
        perf_report.append(
            {
                "run_id": run["id"],
                "created_at": run["created_at"],
                "url": run["html_url"],
                "events": events,
            }
        )

    rollback_report = []
    for run in rollback_runs:
        if run.get("conclusion") != "success":
            continue
        artifacts = api_get(
            f"https://api.github.com/repos/{owner}/{name}/actions/runs/{run['id']}/artifacts",
            token,
        ).get("artifacts", [])
        target = next((a for a in artifacts if a["name"].startswith("rollback-metrics-")), None)
        if not target:
            continue
        data = download_zip(target["archive_download_url"], token)
        with zipfile.ZipFile(BytesIO(data), "r") as zf:
            file_name = next((n for n in zf.namelist() if n.endswith("rollback-metrics.json")), None)
            if not file_name:
                continue
            content = json.loads(zf.read(file_name).decode("utf-8", errors="replace"))
        rollback_report.append(
            {
                "run_id": run["id"],
                "created_at": run["created_at"],
                "url": run["html_url"],
                "success_rate_pct": content["summary"]["rollback_success_rate_pct"],
            }
        )

    ls_values = []
    show_values = []
    for item in perf_report:
        ls_values.extend(item["events"].get("ls", []))
        show_values.extend(item["events"].get("show", []))

    show_p95 = percentile(show_values, 0.95)
    ls_avg = (sum(ls_values) / len(ls_values)) if ls_values else None
    rollback_values = [x["success_rate_pct"] for x in rollback_report]
    rollback_avg = (
        (sum(rollback_values) / len(rollback_values)) if rollback_values else None
    )

    summary = {
        "generated_at_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "thresholds": {
            "ls_threshold_ms": ls_threshold,
            "show_p95_threshold_ms": show_p95_threshold,
            "rollback_threshold_pct": rollback_threshold,
        },
        "aggregate": {
            "ls_avg_ms": ls_avg,
            "show_p95_ms": show_p95,
            "rollback_avg_pct": rollback_avg,
        },
        "perf_runs": perf_report,
        "rollback_runs": rollback_report,
    }

    with open(os.path.join(out_dir, "baseline-trend.json"), "w", encoding="utf-8") as f:
        json.dump(summary, f, ensure_ascii=False, indent=2)

    def fmt(v):
        return "N/A" if v is None else f"{v:.2f}" if isinstance(v, float) else str(v)

    lines = [
        "# Baseline Trend",
        "",
        f"- generated_at_utc: {summary['generated_at_utc']}",
        f"- lookback: {lookback}",
        "",
        "## Aggregate",
        "",
        f"- ls_avg_ms: {fmt(ls_avg)} (threshold <= {ls_threshold})",
        f"- show_p95_ms: {fmt(show_p95)} (threshold <= {show_p95_threshold})",
        f"- rollback_avg_pct: {fmt(rollback_avg)} (threshold >= {rollback_threshold})",
        "",
        "## Perf Baseline Recent Runs",
        "",
    ]
    for item in perf_report:
        ls_last = item["events"].get("ls", [])
        show_last = item["events"].get("show", [])
        lines.append(
            f"- run {item['run_id']} @ {item['created_at']}: ls={ls_last[-1] if ls_last else 'N/A'}ms, show={show_last[-1] if show_last else 'N/A'}ms, [link]({item['url']})"
        )

    lines.extend(["", "## Rollback Metrics Recent Runs", ""])
    for item in rollback_report:
        lines.append(
            f"- run {item['run_id']} @ {item['created_at']}: rollback_success_rate={item['success_rate_pct']}%, [link]({item['url']})"
        )

    with open(os.path.join(out_dir, "baseline-trend.md"), "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")

    print("[baseline-trend] generated artifacts/baseline-trend.md and baseline-trend.json")


if __name__ == "__main__":
    try:
        run()
    except Exception as e:
        print(f"[baseline-trend] failed: {e}", file=sys.stderr)
        raise
