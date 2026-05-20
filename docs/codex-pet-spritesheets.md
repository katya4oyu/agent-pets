# Codex Pet Spritesheets

Codex-compatible pet assets use a fixed atlas and a small manifest.

## Package

```text
${CODEX_HOME:-$HOME/.codex}/pets/<pet-name>/pet.json
${CODEX_HOME:-$HOME/.codex}/pets/<pet-name>/spritesheet.webp
```

`pet.json`:

```json
{
  "id": "pet-name",
  "displayName": "Pet Name",
  "description": "One short sentence.",
  "spritesheetPath": "spritesheet.webp"
}
```

## Atlas Geometry

- Format: PNG or WebP
- Size: `1536x1872`
- Grid: `8` columns by `9` rows
- Cell: `192x208`
- Background: transparent
- Unused cells: fully transparent

To render a frame, crop:

```text
x = column * 192
y = row * 208
w = 192
h = 208
```

The frontend can implement this with canvas drawing or with CSS
`background-position`.

## Rows

| Row | State | Columns | Durations |
| --- | --- | --- | --- |
| 0 | `idle` | 0-5 | `280,110,110,140,140,320` |
| 1 | `running-right` | 0-7 | `120` each, final `220` |
| 2 | `running-left` | 0-7 | `120` each, final `220` |
| 3 | `waving` | 0-3 | `140` each, final `280` |
| 4 | `jumping` | 0-4 | `140` each, final `280` |
| 5 | `failed` | 0-7 | `140` each, final `240` |
| 6 | `waiting` | 0-5 | `150` each, final `260` |
| 7 | `running` | 0-5 | `120` each, final `220` |
| 8 | `review` | 0-5 | `150` each, final `280` |

`idle` column `0` is the best static frame for reduced motion.

## App State Mapping

Agent Pets app states are not the same as Codex pet rows. A reasonable first
mapping is:

| App state | Pet animation |
| --- | --- |
| `thinking` | `review` |
| `running` | `running` |
| `editing` | `review` |
| `waiting_approval` | `waving` |
| `done` | `idle` |
| `error` | `failed` |

Asset validation should check atlas dimensions, alpha channel, non-empty used
cells, fully transparent unused cells, and obvious background-removal failures.
