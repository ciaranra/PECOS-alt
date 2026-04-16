# Surface Sweep Examples

This directory contains the current rotated-surface-code memory sweep tooling:

- `native_dem_threshold_sweep.py`: runs X/Z memory experiments, fits a per-round
  logical error rate, and writes plots plus optional JSON/HTML reports.
- `surface_sweep_report.py`: rebuilds an HTML dashboard from already-generated
  sweep artifacts without rerunning simulations.

## Typical Workflow

Run these commands from the PECOS repo root using your normal project Python
environment.

```bash
python examples/surface/native_dem_threshold_sweep.py \
  --distances 3 5 7 9 \
  --error-rates 0.004 0.006 0.008 0.01 \
  --bases X Z \
  --shots 1000 \
  --sample-backend native_sampler \
  --save-json --save-html \
  --output-dir /tmp/pecos_surface_highshot_sweep
```

This assumes the standard PECOS Python/runtime setup is already available and
that the Selene/native-sampler pieces required by these examples are present.
Use the project's normal setup flow before running these examples if needed.

Run a full native-sampler sweep with the configuration we have been using:

Notes:

- The default duration schedule now uses about four evenly spaced integer round
  counts over `r in [2d, 3d]` for each distance.
- If we need to push a bit past `3d`, use `--duration-max-multiplier ...` or
  provide an explicit `--duration-multipliers ...` list.
- The fixed-`p` duration plots show fitted duration curves as lines and
  observed logical error rates as points with 95% Wilson intervals.

Open the generated report directly from the sweep run:

```bash
python examples/surface/native_dem_threshold_sweep.py \
  --distances 3 5 7 9 \
  --error-rates 0.004 0.006 0.008 0.01 \
  --bases X Z \
  --shots 1000 \
  --sample-backend native_sampler \
  --save-json --save-html --open-html \
  --output-dir /tmp/pecos_surface_highshot_sweep
```

## Rebuild A Report From Existing Artifacts

If the SVGs and JSON already exist, rebuild the dashboard without rerunning the
simulations:

```bash
python examples/surface/surface_sweep_report.py \
  --input-dir /tmp/pecos_surface_highshot_sweep
```

To rebuild and open it in the browser:

```bash
python examples/surface/surface_sweep_report.py \
  --input-dir /tmp/pecos_surface_highshot_sweep \
  --open
```

## Small Smoke Run

For a quick sanity check before launching a heavier sweep:

```bash
python examples/surface/native_dem_threshold_sweep.py \
  --distances 3 5 \
  --error-rates 0.006 \
  --bases X Z \
  --shots 200 \
  --sample-backend native_sampler \
  --save-json --save-html \
  --output-dir /tmp/pecos_surface_fitcurve_check
```

This is useful for validating that:

- Selene/native sampler execution is working
- plots and the HTML dashboard are being generated
- the fixed-`p` duration panels look sensible before spending time on a large
  run
