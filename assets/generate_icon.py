# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""Generate InterWeave's icon set from one geometry.

Two strands, teal and amber, braid across agent-fabric's navy tile and
cross twice: amber passes over at the left crossing, teal at the right.
The palette, the tile and the 62-over-48 casing come from agent-fabric's
docs/assets/agent-fabric-icon.svg (9914eeb4); the strands here are 40 in
54, thinner, because two curves share the space its straight lines had.

Run from the repository root:  python3 assets/generate_icon.py
It rewrites every file listed in assets/README.md. The PNG needs
rsvg-convert (librsvg); the vector files need nothing.
"""
import math
import os
import subprocess

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
NAVY0, NAVY1, AMBER, TEAL = '#232c63', '#141a3a', '#ffb547', '#2ec4b6'
W, CASE = 40, 54
X0, X1, AMP = 104, 408, 104
CROSS_OVER = (X0 + (X1 - X0) * .75, 256)  # teal passes over here
# The casing is clipped tighter than the colour: cut at the same radius,
# the two anti-aliased edges leave a visible seam across the strand.
R_CASE, R_COL = 66, 76
SPDX = '<!-- SPDX-License-Identifier: Apache-2.0 -->'
RES = 'apps/human-android/android/app/src/main/res'


def wave(sign, n=96):
    return [(X0 + (X1 - X0) * i / n, 256 + sign * AMP * math.cos(2 * math.pi * i / n)) for i in range(n + 1)]


UP, DN = wave(-1), wave(1)  # teal, amber


def d(pts):
    return 'M' + ' L'.join(f'{x:.1f} {y:.1f}' for x, y in pts)


def put(rel, text):
    path = os.path.join(ROOT, rel)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f:
        f.write(text)
    return path


def svg_master():
    def s(p, c, w):
        return (f'<path d="{d(p)}" fill="none" stroke="{c}" stroke-width="{w}" '
                'stroke-linecap="round" stroke-linejoin="round"/>')
    cx, cy = CROSS_OVER
    return ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512" '
            f'role="img" aria-label="InterWeave">\n  {SPDX}\n  <title>InterWeave</title>\n'
            f'  <defs><linearGradient id="bg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="{NAVY0}"/>'
            f'<stop offset="1" stop-color="{NAVY1}"/></linearGradient>'
            f'<clipPath id="kc"><circle cx="{cx:.1f}" cy="{cy}" r="{R_CASE}"/></clipPath>'
            f'<clipPath id="kq"><circle cx="{cx:.1f}" cy="{cy}" r="{R_COL}"/></clipPath></defs>\n'
            '  <rect x="16" y="16" width="480" height="480" rx="108" fill="url(#bg)"/>\n'
            f'  {s(UP, NAVY1, CASE)}{s(UP, TEAL, W)}{s(DN, NAVY1, CASE)}{s(DN, AMBER, W)}'
            f'<g clip-path="url(#kc)">{s(UP, NAVY1, CASE)}</g><g clip-path="url(#kq)">{s(UP, TEAL, W)}</g>\n</svg>\n')


def circle(cx, cy, r):
    return f'M{cx - r:.1f},{cy} a{r},{r} 0 1,0 {2 * r},0 a{r},{r} 0 1,0 {-2 * r},0'


def vpath(pts, colour, w):
    return (f'        <path android:pathData="{d(pts)}" android:strokeColor="{colour}" android:strokeWidth="{w:g}"\n'
            '            android:strokeLineCap="round" android:strokeLineJoin="round" android:fillColor="#00000000"/>\n')


def vector(inner, size_dp, scale):
    return ('<?xml version="1.0" encoding="utf-8"?>\n' + SPDX + '\n'
            '<vector xmlns:android="http://schemas.android.com/apk/res/android"\n'
            f'    android:width="{size_dp}dp" android:height="{size_dp}dp" android:viewportWidth="512" android:viewportHeight="512">\n'
            f'    <group android:pivotX="256" android:pivotY="256" android:scaleX="{scale:.4f}" android:scaleY="{scale:.4f}">\n'
            + inner + '    </group>\n</vector>\n')


def main():
    master = put('assets/interweave-icon.svg', svg_master())
    png = os.path.join(ROOT, 'crates/human/ui-slint/ui/assets/interweave-icon-256.png')
    os.makedirs(os.path.dirname(png), exist_ok=True)
    subprocess.run(['rsvg-convert', '-w', '256', '-h', '256', master, '-o', png], check=True)

    # Launcher: the strands, casing included, about 60 dp wide in a 108 dp layer, inside the 66 dp safe zone.
    span = (X1 - X0) + CASE
    launcher = (60 / 108 * 512) / span
    cx, cy = CROSS_OVER
    fg = (vpath(UP, NAVY1, CASE) + vpath(UP, TEAL, W) + vpath(DN, NAVY1, CASE) + vpath(DN, AMBER, W)
          + f'        <group>\n        <clip-path android:pathData="{circle(cx, cy, R_CASE)}"/>\n'
          + vpath(UP, NAVY1, CASE) + '        </group>\n'
          + f'        <group>\n        <clip-path android:pathData="{circle(cx, cy, R_COL)}"/>\n'
          + vpath(UP, TEAL, W) + '        </group>\n')
    put(f'{RES}/drawable/ic_launcher_foreground.xml', vector(fg, 108, launcher))
    # One tone (themed icons, the status bar): the crossings merge, the silhouette carries the mark.
    flat = vpath(UP, '#FFFFFFFF', W) + vpath(DN, '#FFFFFFFF', W)
    put(f'{RES}/drawable/ic_launcher_monochrome.xml', vector(flat, 108, launcher))
    put(f'{RES}/drawable/ic_stat_interweave.xml', vector(flat, 24, (20 / 24 * 512) / (span - CASE + W)))
    put(f'{RES}/drawable/ic_launcher_background.xml',
        '<?xml version="1.0" encoding="utf-8"?>\n' + SPDX + '\n'
        '<vector xmlns:android="http://schemas.android.com/apk/res/android" xmlns:aapt="http://schemas.android.com/aapt"\n'
        '    android:width="108dp" android:height="108dp" android:viewportWidth="108" android:viewportHeight="108">\n'
        '    <path android:pathData="M0,0h108v108h-108z">\n        <aapt:attr name="android:fillColor">\n'
        '            <gradient android:type="linear" android:startX="0" android:startY="0" android:endX="108" android:endY="108"\n'
        f'                android:startColor="{NAVY0}" android:endColor="{NAVY1}"/>\n        </aapt:attr>\n    </path>\n</vector>\n')
    adaptive = ('<?xml version="1.0" encoding="utf-8"?>\n' + SPDX + '\n'
                '<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">\n'
                '    <background android:drawable="@drawable/ic_launcher_background"/>\n'
                '    <foreground android:drawable="@drawable/ic_launcher_foreground"/>\n'
                '    <monochrome android:drawable="@drawable/ic_launcher_monochrome"/>\n</adaptive-icon>\n')
    put(f'{RES}/mipmap-anydpi-v26/ic_launcher.xml', adaptive)
    put(f'{RES}/mipmap-anydpi-v26/ic_launcher_round.xml', adaptive)


if __name__ == '__main__':
    main()
