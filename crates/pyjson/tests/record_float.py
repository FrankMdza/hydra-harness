"""Refresh build 223's float string results with CPython 3.14.7.

Run: python3 crates/pyjson/tests/record_float.py
The fixture carries the input strings; only the recorded results are replaced.
"""
import json
import struct
import sys
from pathlib import Path

assert sys.version_info[:3] == (3, 14, 7), sys.version
path = Path(__file__).resolve().parent / 'fixtures/float.json'
rows = []
for text, _ in json.loads(path.read_text()):
    try:
        value = float(text)
        expected = 'nan' if value != value else struct.pack('>d', value).hex()
    except ValueError:
        expected = None
    rows.append([text, expected])
path.write_text(json.dumps(rows, ensure_ascii=True, indent=2) + '\n')
