#!/usr/bin/env python3
"""Draws the DiaryCraft icon and writes every size the packages need.

The icon is original artwork made for DiaryCraft: a notebook page with a
bookmark on a blue tile, drawn with Pillow shapes (no fonts, no third-party art). Outputs, committed:

  assets/app-icon/diarycraft.svg        reference drawing (same geometry)
  assets/app-icon/hicolor/<n>x<n>/apps/it.mauromazzieri.diarycraft.png
  assets/app-icon/diarycraft-1024.png   source of the macOS .icns (iconutil, at packaging time)
  assets/app-icon/diarycraft.ico        Windows (16-256)
  assets/app-icon/window-64.rgba          raw RGBA for the window icon (include_bytes!)

Run from the repository root: python3 tools/icons.py
"""
from pathlib import Path
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "assets" / "app-icon"
APP_ID = "it.mauromazzieri.diarycraft"
S = 1024  # design grid
BG = (47, 75, 124, 255)
FG = (255, 255, 255, 255)
LINE = (183, 196, 220, 255)
ACCENT = (232, 93, 76, 255)


def draw(size_px: int, margin: float = 0.0) -> Image.Image:
    """The icon at `size_px`, drawn at 4x and reduced for smooth edges.
    `margin`: transparent border as a fraction of the side (macOS grid)."""
    k = 4
    big = S * k
    img = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    m = int(big * margin)
    side = big - 2 * m
    u = side / S  # design unit

    def p(x, y):
        return (m + x * u, m + y * u)

    w = lambda n: max(1, int(n * u))
    d.rounded_rectangle([p(0, 0), p(S, S)], radius=int(220 * u), fill=BG)
    # the page of a notebook
    d.rounded_rectangle([p(262, 176), p(782, 848)], radius=w(40), fill=FG)
    # spine: a band down the left edge
    d.rounded_rectangle([p(262, 176), p(350, 848)], radius=w(40), fill=LINE)
    d.rectangle([p(310, 176), p(350, 848)], fill=LINE)
    # written lines, the last one short
    for i, y in enumerate((400, 490, 580, 670)):
        x2 = 600 if i == 3 else 700
        d.rounded_rectangle([p(410, y), p(x2, y + 26)], radius=w(13), fill=BG)
    # bookmark ribbon hanging from the top edge
    d.polygon([p(616, 150), p(696, 150), p(696, 330), p(656, 290), p(616, 330)], fill=ACCENT)
    return img.resize((size_px, size_px), Image.LANCZOS)


SVG = f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {S} {S}">
  <!-- DiaryCraft icon: reference drawing of tools/icons.py (MIT OR Apache-2.0). -->
  <rect width="{S}" height="{S}" rx="220" fill="#2F4B7C"/>
  <rect x="262" y="176" width="520" height="672" rx="40" fill="#FFFFFF"/>
  <rect x="262" y="176" width="88" height="672" rx="40" fill="#B7C4DC"/>
  <rect x="310" y="176" width="40" height="672" fill="#B7C4DC"/>
  <g fill="#2F4B7C">
    <rect x="410" y="400" width="290" height="26" rx="13"/>
    <rect x="410" y="490" width="290" height="26" rx="13"/>
    <rect x="410" y="580" width="290" height="26" rx="13"/>
    <rect x="410" y="670" width="190" height="26" rx="13"/>
  </g>
  <path d="M616 150 H696 V330 L656 290 L616 330 Z" fill="#E85D4C"/>
</svg>
"""


def mobile() -> None:
    """The phone apps' icons; Apple's has no transparency and no rounded corners."""
    res = ROOT / "mobile" / "android" / "app" / "src" / "main" / "res"
    for name, n in (("mdpi", 48), ("hdpi", 72), ("xhdpi", 96), ("xxhdpi", 144), ("xxxhdpi", 192)):
        d = res / f"mipmap-{name}"
        d.mkdir(parents=True, exist_ok=True)
        draw(n, margin=0.04).save(d / "ic_launcher.png", optimize=True)
    ios = ROOT / "mobile" / "ios" / "DiaryCraft" / "Assets.xcassets" / "AppIcon.appiconset"
    square = Image.new("RGB", (S, S), BG[:3])
    icon = draw(S)
    square.paste(icon, mask=icon.getchannel("A"))
    square.save(ios / "icon-1024.png", optimize=True)


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "diarycraft.svg").write_text(SVG)
    for n in (16, 24, 32, 48, 64, 128, 256, 512):
        d = OUT / "hicolor" / f"{n}x{n}" / "apps"
        d.mkdir(parents=True, exist_ok=True)
        draw(n).save(d / f"{APP_ID}.png", optimize=True)
    # Apple's grid leaves a transparent margin around the tile.
    draw(1024, margin=100 / 1024).save(OUT / "diarycraft-1024.png", optimize=True)
    draw(256).save(OUT / "diarycraft.ico", sizes=[(n, n) for n in (16, 20, 24, 32, 40, 48, 64, 128, 256)])
    (OUT / "window-64.rgba").write_bytes(draw(64).tobytes())
    mobile()


if __name__ == "__main__":
    main()
