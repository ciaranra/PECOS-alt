"""Subprocess-based Selene engine for PECOS integration."""

import logging
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import yaml

logger = logging.getLogger(__name__)


@dataclass
class SeleneSubprocessConfig:
    """Configuration for running Selene as a subprocess."""

    executable_path: Path
    working_dir: Path
    num_qubits: int
    shots: int = 1
    runtime_plugin: Path | None = None
    simulator: str = "Quest"
    verbose: bool = False


class SeleneSubprocessEngine:
    """Runs Selene as a subprocess and captures results.

    This engine:
    1. Builds a Selene executable from Guppy/HUGR
    2. Runs it as a subprocess with proper configuration
    3. Captures and parses the results
    """

    def __init__(self, config: SeleneSubprocessConfig) -> None:
        """Initialize engine with configuration."""
        self.config = config
        self.results = []

    def run(self) -> list[dict[str, Any]]:
        """Run the Selene executable and collect results."""
        # Create configuration file for Selene
        config_path = self.config.working_dir / "selene_config.yaml"

        # Build configuration
        selene_config = {
            "n_qubits": self.config.num_qubits,
            "shots": {
                "count": self.config.shots,
                "offset": 0,
                "increment": 1,
            },
            "simulator": {
                "name": f"selene_sim.{self.config.simulator}",
                # Don't include "file" key if None - Selene will use bundled
                "args": [],
            },
            "error_model": {
                "name": "selene_sim.IdealErrorModel",
                # Don't include "file" key if None
                "args": [],
            },
            "runtime": {
                "name": "selene_sim.SimpleRuntime",
                # Don't include "file" key if None
                "args": [],
            },
            "event_hooks": {},
            "output_stream": "stdout",  # Output to stdout for capture
            "artifact_dir": str(self.config.working_dir / "artifacts"),
        }

        # If we have a custom runtime plugin, use it
        if self.config.runtime_plugin and self.config.runtime_plugin.exists():
            selene_config["runtime"] = {
                "name": "pecos_selene_plugins.ByteMessageSimulatorFactory",
                "file": str(self.config.runtime_plugin),
                "args": [],
            }

        # Write configuration
        with config_path.open("w") as f:
            yaml.dump(selene_config, f)

        logger.info("Running Selene executable: %s", self.config.executable_path)
        logger.debug("Configuration: %s", selene_config)

        # Run Selene executable
        try:
            # Don't use text=True as the output might contain binary data
            result = subprocess.run(
                [str(self.config.executable_path), "--configuration", str(config_path)],
                check=False,
                capture_output=True,
                text=False,  # Binary mode
                cwd=str(self.config.working_dir),
                timeout=30,  # 30 second timeout
            )
        except subprocess.TimeoutExpired as e:
            logger.exception("Selene execution timed out")
            msg = "Selene execution timed out after 30 seconds"
            raise RuntimeError(msg) from e
        except Exception:
            logger.exception("Error running Selene")
            raise
        else:
            # Process result if no exceptions occurred
            if result.returncode != 0:
                logger.error("Selene returned error code %s", result.returncode)
                stderr_text = result.stderr.decode("utf-8", errors="replace")
                logger.error("Stderr: %s", stderr_text)
                msg = f"Selene execution failed: {stderr_text}"
                raise RuntimeError(msg)

            # Parse stdout for results (decode from bytes)
            stdout_text = result.stdout.decode("utf-8", errors="replace")
            results = self._parse_results(stdout_text)

            if self.config.verbose:
                logger.info("Stdout: %s", stdout_text)
                stderr_text = result.stderr.decode("utf-8", errors="replace")
                logger.info("Stderr: %s", stderr_text)

            return results

    def _parse_results(self, output: str) -> list[dict[str, Any]]:
        """Parse Selene output for results."""
        results = []
        current_shot = {}

        for line in output.split("\n"):
            stripped_line = line.strip()
            if not stripped_line:
                continue

            # Look for result tags
            if stripped_line.startswith("USER:INT:"):
                # Parse user output: USER:INT:name:value
                parts = stripped_line.split(":")
                if len(parts) >= 4:
                    name = parts[2]
                    try:
                        value = int(parts[3])
                        current_shot[name] = value
                    except ValueError:
                        logger.warning("Could not parse value: %s", parts[3])

            elif stripped_line.startswith("SHOT:END"):
                # End of shot, save results
                if current_shot:
                    results.append(current_shot)
                    current_shot = {}

        # Save any remaining results
        if current_shot:
            results.append(current_shot)

        return results


def run_selene_subprocess(
    executable_path: Path,
    num_qubits: int = 10,
    shots: int = 1,
    runtime_plugin: Path | None = None,
    *,
    verbose: bool = False,
) -> list[dict[str, Any]]:
    """Convenience function to run Selene as a subprocess.

    Args:
        executable_path: Path to the Selene executable
        num_qubits: Number of qubits to simulate
        shots: Number of shots to run
        runtime_plugin: Optional path to custom runtime plugin
        verbose: Whether to print verbose output

    Returns:
        List of result dictionaries, one per shot
    """
    working_dir = executable_path.parent

    config = SeleneSubprocessConfig(
        executable_path=executable_path,
        working_dir=working_dir,
        num_qubits=num_qubits,
        shots=shots,
        runtime_plugin=runtime_plugin,
        verbose=verbose,
    )

    engine = SeleneSubprocessEngine(config)
    return engine.run()
