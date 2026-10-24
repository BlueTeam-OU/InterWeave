---
role: "rust-ui-dev"
class: solution
topic: "ci-fonts-dejavu-only"
description: "InterWeave CI's rust job has only fonts-dejavu-core; a ui-slint layout test can pass here and fail there; reproduce with a DejaVu-only fontconfig"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 91e7451852ccf8b1
---

## InterWeave CI's rust job has only fonts-dejavu-core; a ui-slint layout test can pass here and fail there; reproduce with a DejaVu-only fontconfig

InterWeave CI (.github/workflows/ci.yml, apt line with `fonts-dejavu-core`) renders Slint tests in DejaVu, which is wider than this host's Adwaita/Cantarell. On #309 (2026-10-19) `the_trust_field_keeps_its_error_under_it_and_its_button_where_it_fits` passed here and failed in CI: an unwrapped origin line set a column's minimum width and pushed Remove trust 3 px past 320 px. Fixed in 21e2a785.

**How to apply:** before pushing a layout change in ui-slint, run its tests under a fontconfig holding only DejaVu. This host has DejaVu in texlive:
`<dir>/usr/share/texlive/texmf-dist/fonts/truetype/public/dejavu</dir>` in a fonts.conf (kept at a scratch path), then `FONTCONFIG_FILE=<it> cargo nextest run -p interweave-human-ui-slint`. To locate an overflow, a temporary probe that prints each element whose x + width passes the window's width (type_name, accessible_label, geometry) finds it in one run. A text in a layout column that does not wrap sets that column's minimum width.

*Observed 2026-10-19 (rust-ui-dev)*
