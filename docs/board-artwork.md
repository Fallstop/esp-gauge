# Board linework provenance

Input supplied by the user: `esp-guage-models.zip`, containing
`ESP Guage PCB_PCB.pdf.obj`, its MTL, and `ESP Guage PCB_PCB.step`.
The OBJ is preserved losslessly as `docs/source-model.obj.gz`; the runtime ships
only a static PNG and small JSON hit-target map. The model SHA256 is recorded
in `desktop/esp_gauge/assets/board-top.json`.

Reproduce from the repository root:

```sh
uv run --with numpy --with pillow tools/render_board.py docs/source-model.obj.gz
```

The renderer projects the actual 186,253 triangles from +Z onto XY, with Y up,
USB on the left and ESP32 antenna on the right. A depth buffer removes hidden
geometry; depth/normal/component boundaries become thin white linework. No
invented component shapes or ongoing 3D rendering are used. Connector hit targets
come from each named J1–J7 mesh group's projected bounds. The PNG is loaded once
and painted only on normal widget exposes/resizes or connector selection.
A future static isometric render can replace this asset without a render loop.

The board group spans 4.5 × 4.0 model units (the export uses centimetres), matching
the PCB outline's 1771.65 × 1574.80 mil span (45 × 40 mm). Connector ordering and
positions agree with PcbDoc SOURCEDESIGNATOR/X/Y records:

| Output | GPIO | Connector | PCB X (mil) | PCB Y (mil) | Top view |
|---|---|---|---|---|---|
| PWM1 | 16 | J1 | 2413.1496 | 1310 | bottom right |
| PWM2 | 17 | J3 | 1698.2748 | 1310 | bottom left of connector trio |
| PWM3 | 18 | J5 | 2055.7122 | 2620 | top middle |
| PWM4 | 19 | J2 | 2055.7122 | 1310 | bottom middle |
| PWM5 | 21 | J4 | 2413.1496 | 2620 | top right |
| PWM6 | 22 | J6 | 1698.2748 | 2620 | top left of connector trio |

Mapping source: `PCB/ESP Guage PCB_Sch.SchDoc` filter rows: PWM1/2/3 labels at
(680,1000)/(680,910)/(680,820) lead to J1/J3/J5; PWM4/5/6 at
(860,1000)/(860,910)/(860,820) lead to J2/J4/J6. The firmware pin mapping is
unchanged from `firmware/pins.md`. Connector reference numbers must not be
assumed to equal PWM numbers. Confirm silkscreen/orientation on the actual board
before connecting a gauge; no physical board test was performed in this session.
