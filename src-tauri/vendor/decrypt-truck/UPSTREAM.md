# Upstream provenance
Source: https://github.com/CoffeSiberian/DecryptTruck
Commit: 4b6a167d7b35a5234bb3dde17ac1532740860791
Version: 1.3.7; license: MIT (LICENSE preserved).
Local changes: library-only manifest (no upstream Windows executable build step); decompressed payload capped at 256 MiB. Application adds format validation, process isolation and timeouts. Further compatibility fixes are documented here.

Compatibility fixes (0.2.0): BSII ordinal dictionaries are resolved per field, not per structure; removed field-name-specific enum rewrites and reject unknown enum indexes. Non-ASCII UTF8 string bytes serialize as SII `\xHH` escapes, matching the reference DLL. Synthetic regression fixtures are in the application tests. No game assets or player saves are included in fixtures.

Anonymous IDs are formatted as minimal leading hexadecimal plus 4-digit groups, including small addresses (e.g. `_nameless.1`), matching the reference decoder.
