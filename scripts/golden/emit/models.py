"""validate.py, loaded once as the model library."""
import importlib.util
from pathlib import Path

_spec = importlib.util.spec_from_file_location("validate", Path(__file__).resolve().parent.parent / "validate.py")
V = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(V)
