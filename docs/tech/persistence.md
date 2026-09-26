# Persistence — saves that never lose data

**Status:** M0 locked. Save format crate default is postcard; final lock at scaffold.

## Versioned binary envelope

```text
magic_u32 | format_version_u16 | payload_len_u64 | payload_bytes | checksum_u64
```

- `SAVE_MAGIC_U32 = 0x554E4956` (ASCII `UNIV`), source: project choice recorded here.
- `format_version_u16` increments on any breaking payload change. Readers reject unknown major versions with a typed error.
- Checksum is a 64-bit non-cryptographic hash of `format_version_u16 || payload_bytes`. Algorithm pinned at scaffold.
- Default serializer candidate is postcard. Final lock happens at scaffold; if verification fails, record `to be locked at scaffold` and keep the envelope stable.

## Decode order

Always `checksum -> version -> parse`. A file that fails checksum is treated as corrupt before any parse runs. A file with an unsupported version is reported, never parsed speculatively.

## Atomic writes

- Write to `save.tmp` in the same directory, flush and sync, then rename to the target name. Rename is the commit point.
- Never write in place. Never leave a half-written target after a crash or power loss.

## Corrupt files

- Quarantine corrupt files; never delete them. Move to `corrupt/` with the failing checksum and version in the filename.
- Never boot-loop. If the newest save fails, the game boots to a safe menu with the quarantine report. It never retries the same file automatically.
- Report exact bytes, expected versus actual checksum, and version in the error shown to logs.

## Never-saved list

- Procedural content (regenerable from seed; see [simulation.md](simulation.md)).
- Render caches (pipelines, compiled shaders, texture uploads).
- Transient profiling spans and frame-time history.
- Debug shell state.

Only seeds, elapsed mission seconds, player state, and placed or dropped objects persist, per [specs.md](../specs.md) section 4.

Related: [../tech.md](../tech.md), [simulation.md](simulation.md), [standards.md](standards.md).
