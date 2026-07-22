#!/usr/bin/env python3
"""
Generates branded NSIS installer artwork in the app's dark/purple theme.

Outputs (into src-tauri/installer/):
  sidebar.bmp  164x314  BMP3  - welcome/finish left panel
  header.bmp   150x57   BMP3  - top-right header banner on inner pages

Palette (from src/theme/colors.ts):
  space #0B0716, spaceElevated #150E28, surface #1C1338
  accentIndigo #6C3CFF, accentBlue #9D5CFF, accentCyan #C77DFF, accentMagenta #E85CD8
"""
import math
import os
from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.normpath(os.path.join(HERE, "..", "src-tauri", "installer"))
os.makedirs(OUT, exist_ok=True)

SPACE = (0x0B, 0x07, 0x16)
SPACE_HI = (0x1B, 0x11, 0x40)
INDIGO = (0x6C, 0x3C, 0xFF)
BLUE = (0x9D, 0x5C, 0xFF)
CYAN = (0xC7, 0x7D, 0xFF)
MAGENTA = (0xE8, 0x5C, 0xD8)
WHITE = (0xF2, 0xEE, 0xFA)


def lerp(a, b, t):
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3))


def radial_bg(w, h, cx, cy, inner, outer, radius):
    """Radial gradient from inner (at center) to outer (at edges)."""
    img = Image.new("RGB", (w, h), outer)
    px = img.load()
    maxd = radius
    for y in range(h):
        for x in range(w):
            d = math.hypot(x - cx, y - cy) / maxd
            d = min(1.0, d)
            px[x, y] = lerp(inner, outer, d)
    return img


def add_glow(img, cx, cy, color, radius, strength=0.55):
    """Additively blend a soft circular glow onto img."""
    px = img.load()
    w, h = img.size
    for y in range(max(0, cy - radius), min(h, cy + radius)):
        for x in range(max(0, cx - radius), min(w, cx + radius)):
            d = math.hypot(x - cx, y - cy) / radius
            if d >= 1.0:
                continue
            f = (1.0 - d) ** 2 * strength
            base = px[x, y]
            px[x, y] = tuple(min(255, int(base[i] + color[i] * f)) for i in range(3))


def draw_mesh(draw, w, h, step=26, color=INDIGO, opacity=40):
    """Faint grid lines echoing the app's MeshBackground."""
    line = tuple(int(c * opacity / 255) for c in color)
    for x in range(0, w, step):
        draw.line([(x, 0), (x, h)], fill=line, width=1)
    for y in range(0, h, step):
        draw.line([(0, y), (w, y)], fill=line, width=1)


def load_font(size, bold=True):
    candidates = [
        r"C:\Windows\Fonts\segoeuib.ttf" if bold else r"C:\Windows\Fonts\segoeui.ttf",
        r"C:\Windows\Fonts\arialbd.ttf" if bold else r"C:\Windows\Fonts\arial.ttf",
    ]
    for c in candidates:
        if os.path.exists(c):
            return ImageFont.truetype(c, size)
    return ImageFont.load_default()


def rounded_logo(size, radius_ratio=0.28):
    """The app's 'I' mark on a vertical indigo->blue gradient, rounded corners."""
    s = size
    tile = Image.new("RGB", (s, s))
    px = tile.load()
    for y in range(s):
        t = y / s
        col = lerp(INDIGO, MAGENTA, t)
        for x in range(s):
            px[x, y] = col
    # rounded mask
    mask = Image.new("L", (s, s), 0)
    md = ImageDraw.Draw(mask)
    md.rounded_rectangle([0, 0, s - 1, s - 1], radius=int(s * radius_ratio), fill=255)
    # draw serif 'I'
    d = ImageDraw.Draw(tile)
    bar_w = int(s * 0.16)
    cap_w = int(s * 0.44)
    cx = s // 2
    top = int(s * 0.24)
    bot = int(s * 0.76)
    # vertical stem
    d.rectangle([cx - bar_w // 2, top, cx + bar_w // 2, bot], fill=WHITE)
    # top & bottom caps
    d.rectangle([cx - cap_w // 2, top, cx + cap_w // 2, top + bar_w], fill=WHITE)
    d.rectangle([cx - cap_w // 2, bot - bar_w, cx + cap_w // 2, bot], fill=WHITE)
    out = Image.new("RGB", (s, s), SPACE)
    out.paste(tile, (0, 0), mask)
    return out, mask


def make_sidebar():
    w, h = 164, 314
    img = radial_bg(w, h, cx=int(w * 0.3), cy=int(h * 0.12), inner=SPACE_HI, outer=SPACE, radius=h * 0.9)
    add_glow(img, int(w * 0.15), int(h * 0.05), INDIGO, 150, 0.5)
    add_glow(img, int(w * 0.9), int(h * 0.35), MAGENTA, 140, 0.35)
    add_glow(img, int(w * 0.5), int(h * 0.95), BLUE, 160, 0.4)
    d = ImageDraw.Draw(img)
    draw_mesh(d, w, h, step=26, color=INDIGO, opacity=34)

    # logo mark
    logo_size = 72
    logo, mask = rounded_logo(logo_size)
    lx = (w - logo_size) // 2
    ly = 46
    # subtle glow behind logo
    add_glow(img, w // 2, ly + logo_size // 2, BLUE, 90, 0.5)
    img.paste(logo, (lx, ly), mask)

    d = ImageDraw.Draw(img)
    # product name
    f_title = load_font(19, bold=True)
    f_sub = load_font(12, bold=False)
    title1 = "Infinity"
    title2 = "Connect"
    tw1 = d.textlength(title1, font=f_title)
    tw2 = d.textlength(title2, font=f_title)
    ty = ly + logo_size + 18
    d.text(((w - tw1) / 2, ty), title1, font=f_title, fill=WHITE)
    d.text(((w - tw2) / 2, ty + 22), title2, font=f_title, fill=CYAN)

    # thin accent divider
    dy = ty + 52
    for i in range(w - 48):
        t = i / (w - 48)
        d.point((24 + i, dy), fill=lerp(INDIGO, MAGENTA, t))
        d.point((24 + i, dy + 1), fill=lerp(INDIGO, MAGENTA, t))

    sub = "VPN"
    sw = d.textlength(sub, font=f_sub)
    d.text(((w - sw) / 2, dy + 10), sub, font=f_sub, fill=(0xB9, 0xA9, 0xE0))

    img.save(os.path.join(OUT, "sidebar.bmp"), "BMP")
    print("  - sidebar.bmp 164x314")


def make_header():
    w, h = 150, 57
    img = radial_bg(w, h, cx=w, cy=0, inner=SPACE_HI, outer=SPACE, radius=w * 1.1)
    add_glow(img, w - 10, 6, INDIGO, 70, 0.5)
    add_glow(img, 10, h, MAGENTA, 60, 0.3)
    d = ImageDraw.Draw(img)
    draw_mesh(d, w, h, step=19, color=INDIGO, opacity=30)

    logo_size = 38
    logo, mask = rounded_logo(logo_size, radius_ratio=0.3)
    ly = (h - logo_size) // 2
    lx = w - logo_size - 10
    add_glow(img, lx + logo_size // 2, ly + logo_size // 2, BLUE, 44, 0.5)
    img.paste(logo, (lx, ly), mask)

    img.save(os.path.join(OUT, "header.bmp"), "BMP")
    print("  - header.bmp 150x57")


if __name__ == "__main__":
    print(f"Generating installer art -> {OUT}")
    make_sidebar()
    make_header()
    print("Done.")
