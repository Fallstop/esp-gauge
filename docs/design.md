# Interface

The board is the navigation. A single inspector follows the selected physical header; there are no dashboards, pages of cards, or save controls. The rear three headers are PWM1–3, left to right, and the front three are PWM4–6.

The mark is a PWM square wave that straightens into a gauge needle: duty cycle in, deflection out. Ink wave, pale-green needle and hub, on a graphite tile a shade lighter than the window so it keeps an edge on dark docks. It is deliberately theme-neutral, because no platform lets a bundled app icon follow light or dark mode (macOS `.icns`, Windows `.ico` and freedesktop `.desktop` icons are each a single fixed image). Sources live in `desktop/src-tauri/icons`: `app.svg` is the master for 48 px and up, `app-small.svg` is a one-pulse simplification for the 16–32 px rasters, and `tray.svg` is the white-on-halo tray glyph, which macOS treats as a template image. The header wordmark uses the same glyph inline in `App.svelte`.

Palette: graphite `#202323`, panel `#252929`, white ink `#e7eae4`, muted ink `#929d97`, rule `#3a4340`, pale green `#b8d5c4`. Calibration alone uses warm amber. Manrope is the primary typeface; IBM Plex Mono identifies physical outputs and readings. All fonts ship with the app.

The supplied Blender line and ambient-occlusion passes are kept intact in `desktop/public/assets`. The line pass is black with transparency; CSS inverts it. The shadow pass is multiplied against the scene background. No external artwork service, browser network request, or regenerated board model is involved.

The six labels trace back to the physical connectors. Selected headers receive a restrained highlight. Values represent commanded electrical position; there is no sensed needle position. Unavailable readings appear as a dash, not zero. Calibration starts at zero and uses a bounded, renewable live preview. Ordinary changes are debounced and acknowledged only after storage on the board.

Physical header targets and their highlights share the exact labelled silhouettes from the supplied `PWM-Targets.svg`, retained in `desktop/src/boardTargets.ts`. Their 5120×2880 viewBox scales with the Blender image, including its scene offset. Overlapping header edges follow the supplied visible outlines; empty space around a connector does not select it. Hovering or keyboard-focusing its label also highlights the corresponding silhouette.

Leader traces approach their own connector without crossing neighbouring housings. The empty inspector uses the supplied `Header.svg` geometry, kept in `desktop/public/assets/header.svg`. Its fourteen original paths sit over graphite housing faces, a dark recess, and pale metal contacts. Editor metadata and the hidden reference image are omitted from the shipped asset.
