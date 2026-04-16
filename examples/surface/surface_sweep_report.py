#!/usr/bin/env python3
"""Build an HTML dashboard from an existing surface sweep output directory.

This lets us browse previously generated SVG/JSON artifacts without rerunning
the sweep itself.

Examples:
    .venv/bin/python examples/surface/surface_sweep_report.py \
        --input-dir /tmp/pecos_surface_real_sweep

    .venv/bin/python examples/surface/surface_sweep_report.py \
        --input-dir /tmp/pecos_surface_real_sweep \
        --open
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any

from native_dem_threshold_sweep import (
    DashboardPlot,
    FitSummary,
    _maybe_open_html_dashboard,
    _write_html_dashboard,
)


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--input-dir",
        type=Path,
        required=True,
        help="Directory containing existing surface sweep SVG and optional JSON artifacts.",
    )
    parser.add_argument(
        "--json-file",
        type=Path,
        default=None,
        help="Optional explicit path to the sweep JSON results file.",
    )
    parser.add_argument(
        "--output-html",
        type=Path,
        default=None,
        help="Optional explicit path for the generated dashboard HTML.",
    )
    parser.add_argument(
        "--open",
        action="store_true",
        help="Open the generated dashboard after writing it.",
    )
    return parser.parse_args()


def _load_json_payload(input_dir: Path, json_file: Path | None) -> tuple[dict[str, Any] | None, Path | None]:
    if json_file is not None:
        payload = json.loads(json_file.read_text())
        return payload, json_file

    json_candidates = sorted(input_dir.glob("*_results.json"))
    if not json_candidates:
        return None, None

    json_path = json_candidates[0]
    payload = json.loads(json_path.read_text())
    return payload, json_path


def _infer_output_html(input_dir: Path, json_path: Path | None, explicit_output: Path | None) -> Path:
    if explicit_output is not None:
        return explicit_output
    if json_path is not None and json_path.name.endswith("_results.json"):
        return input_dir / json_path.name.replace("_results.json", "_dashboard.html")
    return input_dir / "surface_sweep_dashboard.html"


def _extract_backend(prefix_text: str, backends: list[str]) -> str:
    for backend in sorted(backends, key=len, reverse=True):
        if prefix_text.endswith(f"_{backend}"):
            return backend
    return prefix_text


def _maybe_parse_rate(rate_text: str) -> float | None:
    try:
        return float(rate_text.replace("p", ".", 1))
    except ValueError:
        return None


def _discover_dashboard_plots(input_dir: Path, *, backends: list[str]) -> list[DashboardPlot]:
    plots: list[DashboardPlot] = []
    for svg_path in sorted(input_dir.glob("*.svg")):
        stem = svg_path.stem

        if stem.endswith("_per_round_overlay"):
            prefix_text = stem[: -len("_per_round_overlay")]
            backend = _extract_backend(prefix_text, backends)
            plots.append(
                DashboardPlot(
                    section="combined",
                    title=f"Per-round logical error rate vs p ({backend})",
                    filename=svg_path.name,
                    backend=backend,
                ),
            )
            continue

        duration_match = re.match(r"^(?P<prefix>.+)_p_(?P<rate>.+)_duration_overlay$", stem)
        if duration_match is not None:
            prefix_text = duration_match.group("prefix")
            backend = _extract_backend(prefix_text, backends)
            rate = _maybe_parse_rate(duration_match.group("rate"))
            rate_label = duration_match.group("rate").replace("p", ".", 1)
            plots.append(
                DashboardPlot(
                    section="duration",
                    title=f"Logical memory error vs duration ({backend}, p={rate_label})",
                    filename=svg_path.name,
                    backend=backend,
                    physical_error_rate=rate,
                ),
            )
            continue

        basis_match = re.match(r"^(?P<prefix>.+)_(?P<basis>x|z)_(?P<metric>per_round|projected_d_rounds)$", stem)
        if basis_match is not None:
            prefix_text = basis_match.group("prefix")
            backend = _extract_backend(prefix_text, backends)
            basis = basis_match.group("basis").upper()
            metric = basis_match.group("metric")
            title = (
                f"{basis}-Basis Fitted Logical Error Rate Per Round ({backend})"
                if metric == "per_round"
                else f"{basis}-Basis Fitted Logical Error Rate Over d Rounds ({backend})"
            )
            plots.append(
                DashboardPlot(
                    section="basis",
                    title=title,
                    filename=svg_path.name,
                    backend=backend,
                    basis=basis,
                ),
            )

    return plots


def _dashboard_args(payload: dict[str, Any] | None) -> argparse.Namespace:
    config = {} if payload is None else dict(payload.get("config", {}))
    return argparse.Namespace(
        distances=config.get("distances", []),
        duration_multipliers=config.get("duration_multipliers", []),
        error_rates=config.get("error_rates", []),
        shots=config.get("shots", "?"),
    )


def _dashboard_summaries(payload: dict[str, Any] | None) -> list[FitSummary]:
    if payload is None:
        return []
    return [FitSummary(**row) for row in payload.get("fit_summaries", [])]


def _dashboard_timing_summary(payload: dict[str, Any] | None) -> dict[str, Any]:
    if payload is None:
        return {"overall_shots_per_second": 0.0}
    return dict(payload.get("timing_summary", {"overall_shots_per_second": 0.0}))


def main() -> int:
    """Build a dashboard from an existing sweep artifact directory."""
    args = _parse_args()
    input_dir = args.input_dir.expanduser().resolve()
    if not input_dir.is_dir():
        msg = f"Input directory does not exist: {input_dir}"
        raise ValueError(msg)

    json_payload, json_path = _load_json_payload(input_dir, args.json_file)
    output_html = _infer_output_html(input_dir, json_path, args.output_html)
    backends = []
    if json_payload is not None:
        backends = list(json_payload.get("config", {}).get("executed_backends", []))

    plots = _discover_dashboard_plots(input_dir, backends=backends)
    if not plots:
        msg = f"No SVG plots found in {input_dir}"
        raise ValueError(msg)

    _write_html_dashboard(
        output_html,
        args=_dashboard_args(json_payload),
        summaries=_dashboard_summaries(json_payload),
        timing_summary=_dashboard_timing_summary(json_payload),
        plots=plots,
        json_filename=None if json_path is None else json_path.name,
    )
    print(f"Wrote HTML dashboard to {output_html}")
    if args.open:
        _maybe_open_html_dashboard(output_html)
        print(f"Opened HTML dashboard at {output_html}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
