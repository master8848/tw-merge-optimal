# Value validators

Every value a Tailwind utility can accept is checked by a **validator** —
a pure `fn(&str) -> bool` in `crates/twm-core/src/values.rs`. They are a
direct port of tailwind-merge's `validators.ts` truth tables (verified against
its `validators.test.ts` in `tests/validators_truth.rs`), plus the extra types
used by `--value(...)` catalog markers.

Validators run **at build time only**: `twm-gen` uses them to decide whether a
candidate class resolves against a wildcard utility's alternatives. The
generated JS carries an equivalent `VT` switch (`generate.rs::VT_CASES`) so
patterns mode can validate unseen classes at runtime.

## Plain value validators

| Validator | Matches | Ported from |
|---|---|---|
| `is_fraction` | `3/4`, `1.5/2.5` | `isFraction` |
| `is_number` | any `f64` parseable (`-1`, `2.5`, `1e3`) | `isNumber` |
| `is_integer` | any `i64` parseable | `isInteger` |
| `is_percent` | number + `%` | `isPercent` |
| `is_tshirt_size` | `(2xl)` etc. — `(digits.)?(xs\|sm\|md\|lg\|xl)` | `isTshirtSize` |
| `is_length_only` | unit length, `calc()/min()/max()/clamp()`, or `0` — but **not** a color function (percentages inside `rgb(...)` would otherwise look like lengths) | `isAnyNonArbitraryLength` (with color-function rejection) |
| `is_shadow` | `inset_`-optional `x_y` offsets with unit or `0` | `isShadow` |
| `is_image` | `url()`, `image()`, `image-set()`, `cross-fade()`, `element()`, gradients | `isImage` |
| `is_any` | anything | `isAny` |
| `is_any_non_arbitrary` | anything that is not `[...]` or `(...)` | `isAnyNonArbitrary` |
| `is_angle` | number or `deg`/`grad`/`rad`/`turn` suffix | `isAngle` |
| `is_time` | number or `ms`/`s` suffix | `isTime` |
| `is_ident` | CSS ident (`-?[a-zA-Z_\\][\w-\\]*`) | `isCustomIdent` |
| `is_position_keyword` | `center`, `top`, `bottom`, `left`, `right`, 8 corner combos | `isPosition` |
| `is_arbitrary_value` | `[...]` (optionally `[type:]value`) | `isArbitraryValue` |
| `is_arbitrary_variable` | `(...)` (optionally `(type:)value`) | `isArbitraryVariable` |
| `is_named_container_query` | `@container/name`, `@container-size/name`, `@container-normal/name` | `isNamedContainerQuery` |

## Arbitrary-value validators (`a-*`)

These validate `[value]` / `[label:value]` strings. With no label, the inner
value is checked with the corresponding plain validator (or always accepted);
with a label, only the label is checked — the `[...]` brackets guarantee it is
arbitrary, so the browser will accept any value:

- `is_arbitrary_length` — no label: `is_length_only`; label: `length`
- `is_arbitrary_number` — label `number`
- `is_arbitrary_integer` — labels `number`/`integer`
- `is_arbitrary_percent` — no label: `%`-number; labels `position`/`percentage`
- `is_arbitrary_fraction` — no label: fraction; labels `number`/`ratio`
- `is_arbitrary_size` — labels `length`/`size`/`bg-size`
- `is_arbitrary_position` — labels `position`/`percentage`
- `is_arbitrary_shadow` — no label: shadow; label `shadow`
- `is_arbitrary_image` — no label: image; labels `image`/`url`
- `is_arbitrary_weight` — no label: any; labels `number`/`weight`
- `is_arbitrary_family_name` — label `family-name`
- `is_arbitrary_angle` / `time` / `ratio` / `ident` / `url` — no label or the
  matching label
- `is_arbitrary_string` / `function` / `custom-property` / `any` — any `[...]`

## Arbitrary-variable validators (`v-*`)

Identical semantics for `(value)` / `(label:value)` — the Tailwind v4 CSS-variable
shorthand (`bg-(--brand)`, `text-(length:--size)`):

`is_arbitrary_variable_length`, `_family_name`, `_position`, `_size`, `_image`,
`_shadow`, `_weight`, `_angle`, `_time`, `_ident`, `_url`, `_percent`,
`_number`, `_fraction`, `_string`, `_custom_property`, `_any`.

## Where each type is used

The `<type>` markers in `vendor/builtin-utilities.css` reference the validator
names 1:1. For example:

```css
@utility p-* {
    padding: --value(--spacing, <length>);
}
```

means `p-<value>` resolves when the value is a `--spacing` theme key (any
number, `px`, arbitrary) **or** an arbitrary length — `p-2`, `p-px`,
`p-[13px]` all resolve; `p-red` does not.

## The JS port

`generate.rs` emits one `VT(case, value)` function plus small helpers
(`N`, `I`, `L`, `AV`, `V`, `F`, `P1`, `P2`, `LU`, `CF`) that mirror these
functions exactly. `tests/validators_truth.rs` pins the Rust truth tables and
`tests/js_parity.rs` runs all corpus cases against the generated JS, so the
two ports cannot drift.
