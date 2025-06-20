# Python Version Compatibility Notes

## Python 3.13 and guppylang

When running tests with Python 3.13, you may encounter deprecation warnings from the `guppylang` library:

```
DeprecationWarning: DesugaredGenerator.__init__ got an unexpected keyword argument 'used_outer_places'.
Support for arbitrary keyword arguments is deprecated and will be removed in Python 3.15.
```

This is a known compatibility issue between guppylang 0.19.1 and Python 3.13's stricter AST node handling.

### Recommendation

For the best experience with PECOS and all optional dependencies, we recommend using **Python 3.12** until guppylang releases an update that fully supports Python 3.13.

### Automatic Handling

Our test configuration (`conftest.py`) automatically:
1. Detects when Python 3.13+ is being used
2. Suppresses the guppylang deprecation warnings
3. Displays a one-time notification about the compatibility issue

### Setting Python Version with uv

To use Python 3.12 with uv:

```bash
uv python pin 3.12
```

This will ensure all uv commands use Python 3.12 for this project.
