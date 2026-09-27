"""Explicit immutable dependencies; never replace globals or invoke old main."""
from pathlib import Path
import sys
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
OLD=HERE.parent/'mqlib_difficulty_qualification'
sys.path.insert(0,str(OLD))
import run as worker
import build as native_build
import analysis as old_analysis
sys.path.pop(0)
assert Path(worker.__file__).resolve()==OLD/'run.py'
assert Path(native_build.__file__).resolve()==OLD/'build.py'
assert Path(old_analysis.__file__).resolve()==OLD/'analysis.py'
