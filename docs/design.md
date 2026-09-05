# Interface

The board is the navigation. A single inspector follows the selected physical header; there are no dashboards, pages of cards, or save controls. The rear three headers are PWM1–3, left to right, and the front three are PWM4–6.

Palette: graphite `#202323`, panel `#252929`, white ink `#e7eae4`, muted ink `#929d97`, rule `#3a4340`, pale green `#b8d5c4`. Calibration alone uses warm amber. Manrope is the primary typeface; IBM Plex Mono identifies physical outputs and readings. All fonts ship with the app.

The supplied Blender line and ambient-occlusion passes are kept intact in `desktop/public/assets`. The line pass is black with transparency; CSS inverts it. The shadow pass is multiplied against the scene background. No external artwork service, browser network request, or regenerated board model is involved.

The six labels trace back to the physical connectors. Selected headers receive a restrained highlight. Values represent commanded electrical position; there is no sensed needle position. Unavailable readings appear as a dash, not zero. Calibration starts at zero and uses a bounded, renewable live preview. Ordinary changes are debounced and acknowledged only after storage on the board.
