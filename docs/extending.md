# Adding sources

A source has a stable identifier, a unit, a full-scale value, and a sampling function. The UI registry in `desktop/src/model.ts` groups sources and supplies labels and descriptions. Settings are independent of transport and use that stable identifier, so existing configurations remain portable.

For a new computer source, add a descriptor to `sources` and insert the raw reading into the map returned by `Metrics::sample` in `desktop/src-tauri/src/metrics.rs`. Omit unavailable readings. The worker normalizes against the channel's scale, clamps to 0–1, and routes values to the correct physical port. Do not add platform shell pipelines for system metrics; use a portable provider or an OS-specific adapter behind the same source ID.

Discoverable sources live in `desktop/src-tauri/src/providers`. Each adapter returns a `Feed` of source descriptors and optional raw values. The desktop renders those descriptors automatically; no new source-picker UI is necessary. Local process/database adapters poll every two seconds, account quotas every minute, and the opt-in public source every five minutes. Keep blocking I/O in these provider threads, use deadlines and size limits, and omit a value on failure. Read local databases with the provided read-only connection and short busy timeout. Do not copy account credentials into configuration.

The channel's `input_min` and `scale` define the raw value range; `(value - input_min) / (scale - input_min)` maps it onto the calibrated endpoints. Firmware 2.2 adds `wave_sine`, `wave_triangle`, `wave_saw` and `wave_square`, with `period_s` (0.1–86,400) and `phase_deg` (0–360). Waveforms share the board's monotonic clock and continue without the PC.

For an independent board source, use an `esp_` identifier and implement its sample in `firmware/src/senses.cpp`. The engine handles calibration, reversal, response, pause and output limits. Slow sensor work must be asynchronous or on a separate task; never block the serial/output loop. Add a corresponding descriptor to the desktop registry. Clock sources are handled separately by the portable clock calculation in `firmware/include/gauge_engine.h`.

Keep unknown fields in configuration so older desktops preserve extension-specific metadata. Increment the protocol/configuration version only for incompatible changes; unknown host sources are preserved and safely rest when the current desktop has no provider.
