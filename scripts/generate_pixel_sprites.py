#!/usr/bin/env python3
import math
import os
import struct
import zlib
from dataclasses import dataclass

SPRITE_W = 20
SPRITE_H = 30
DEFAULT_STYLE_KEY = "style10_painted"
HIGHRES_SCALE = 10


def clamp(value, low=0, high=255):
    return max(low, min(high, int(value)))


def shade(color, factor):
    alpha = color[3] if len(color) > 3 else 255
    return tuple(clamp(c * factor) for c in color[:3]) + (alpha,)


def rgba(color, alpha=255):
    return (color[0], color[1], color[2], alpha)


def blend(a, b, t):
    return tuple(clamp(a[i] * (1 - t) + b[i] * t) for i in range(3)) + (255,)


def apply_style_color(color, style):
    if len(color) == 3:
        r, g, b = color
        a = 255
    else:
        r, g, b, a = color
    r = r * style.brightness
    g = g * style.brightness
    b = b * style.brightness
    r = (r - 128) * style.contrast + 128
    g = (g - 128) * style.contrast + 128
    b = (b - 128) * style.contrast + 128
    gray = 0.3 * r + 0.59 * g + 0.11 * b
    r = gray + (r - gray) * style.saturation
    g = gray + (g - gray) * style.saturation
    b = gray + (b - gray) * style.saturation
    if style.tint is not None and style.tint_strength > 0:
        r = r * (1 - style.tint_strength) + style.tint[0] * style.tint_strength
        g = g * (1 - style.tint_strength) + style.tint[1] * style.tint_strength
        b = b * (1 - style.tint_strength) + style.tint[2] * style.tint_strength
    if style.posterize_levels and style.posterize_levels > 1:
        step = 255 / (style.posterize_levels - 1)
        r = round(r / step) * step
        g = round(g / step) * step
        b = round(b / step) * step
    return (clamp(r), clamp(g), clamp(b), clamp(a))


def make_ramp(base, style):
    base = rgba(base)
    return {
        "base": apply_style_color(base, style),
        "dark": apply_style_color(shade(base, style.dark_factor)[:3], style),
        "darker": apply_style_color(shade(base, style.darker_factor)[:3], style),
        "light": apply_style_color(shade(base, style.light_factor)[:3], style),
        "highlight": apply_style_color(shade(base, style.highlight_factor)[:3], style),
    }


class Canvas:
    def __init__(self, width, height, bg=(0, 0, 0, 0)):
        self.width = width
        self.height = height
        self.pixels = bytearray(width * height * 4)
        if bg[3] > 0:
            self.rect(0, 0, width, height, bg)

    def set_px(self, x, y, color):
        if x < 0 or y < 0 or x >= self.width or y >= self.height:
            return
        r, g, b, a = color
        idx = (y * self.width + x) * 4
        self.pixels[idx : idx + 4] = bytes((r, g, b, a))

    def get_px(self, x, y):
        if x < 0 or y < 0 or x >= self.width or y >= self.height:
            return (0, 0, 0, 0)
        idx = (y * self.width + x) * 4
        return tuple(self.pixels[idx : idx + 4])

    def rect(self, x, y, w, h, color):
        for iy in range(y, y + h):
            for ix in range(x, x + w):
                self.set_px(ix, iy, color)

    def line(self, x0, y0, x1, y1, color):
        dx = abs(x1 - x0)
        dy = -abs(y1 - y0)
        sx = 1 if x0 < x1 else -1
        sy = 1 if y0 < y1 else -1
        err = dx + dy
        x, y = x0, y0
        while True:
            self.set_px(x, y, color)
            if x == x1 and y == y1:
                break
            e2 = 2 * err
            if e2 >= dy:
                err += dy
                x += sx
            if e2 <= dx:
                err += dx
                y += sy

    def blit(self, x, y, width, height, pixels):
        for iy in range(height):
            for ix in range(width):
                src_idx = (iy * width + ix) * 4
                sr, sg, sb, sa = pixels[src_idx : src_idx + 4]
                if sa == 0:
                    continue
                dst_x = x + ix
                dst_y = y + iy
                if dst_x < 0 or dst_y < 0 or dst_x >= self.width or dst_y >= self.height:
                    continue
                dst_idx = (dst_y * self.width + dst_x) * 4
                dr, dg, db, da = self.pixels[dst_idx : dst_idx + 4]
                alpha = sa / 255.0
                out_r = clamp(sr * alpha + dr * (1 - alpha))
                out_g = clamp(sg * alpha + dg * (1 - alpha))
                out_b = clamp(sb * alpha + db * (1 - alpha))
                out_a = clamp(255 * (alpha + (da / 255.0) * (1 - alpha)))
                self.pixels[dst_idx : dst_idx + 4] = bytes((out_r, out_g, out_b, out_a))


@dataclass
class Style:
    key: str
    detail: int
    outline: bool
    shading: bool
    outline_color: tuple
    light_factor: float
    dark_factor: float
    darker_factor: float
    highlight_factor: float
    tint: tuple
    tint_strength: float
    saturation: float
    brightness: float
    contrast: float
    posterize_levels: int
    dither: bool
    dither_strength: int
    noise: bool
    noise_strength: int
    blur_radius: int
    smooth_scale: bool
    sprite_scale: int
    weapon_scale: int
    edge_soften: bool
    edge_soften_strength: float
    gradient_top: float
    gradient_bottom: float
    render_mode: str = "classic"
    weapon_mode: str = "classic"


@dataclass
class SpriteSpec:
    race_id: str
    skin: tuple
    hair: tuple
    eyes: tuple
    tunic: tuple
    pants: tuple
    boots: tuple
    accent: tuple
    height: float
    build: float
    ear: str
    nose: str
    brow: str
    hair_style: str
    freckles: bool
    rugged: bool
    tattoo: tuple
    face_style: str = "classic"


@dataclass
class WeaponSpec:
    key: str
    width: int
    height: int
    draw_fn: object


DEFAULT_OUTLINE = rgba((24, 22, 28))


def apply_outline(canvas, outline_color):
    w, h = canvas.width, canvas.height
    new_pixels = bytearray(canvas.pixels)
    for y in range(h):
        for x in range(w):
            idx = (y * w + x) * 4
            if canvas.pixels[idx + 3] != 0:
                continue
            for nx, ny in ((x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)):
                if nx < 0 or ny < 0 or nx >= w or ny >= h:
                    continue
                nidx = (ny * w + nx) * 4
                if canvas.pixels[nidx + 3] != 0:
                    new_pixels[idx : idx + 4] = bytes(outline_color)
                    break
    canvas.pixels = new_pixels


def draw_rect_outline(canvas, x, y, w, h, color):
    if w <= 0 or h <= 0:
        return
    for ix in range(x, x + w):
        canvas.set_px(ix, y, color)
        canvas.set_px(ix, y + h - 1, color)
    for iy in range(y, y + h):
        canvas.set_px(x, iy, color)
        canvas.set_px(x + w - 1, iy, color)


def draw_circle(canvas, cx, cy, r, color):
    r2 = r * r
    for y in range(cy - r, cy + r + 1):
        for x in range(cx - r, cx + r + 1):
            dx = x - cx
            dy = y - cy
            if dx * dx + dy * dy <= r2:
                canvas.set_px(x, y, color)


def draw_circle_outline(canvas, cx, cy, r, color):
    r2 = r * r
    for y in range(cy - r, cy + r + 1):
        for x in range(cx - r, cx + r + 1):
            dx = x - cx
            dy = y - cy
            dist2 = dx * dx + dy * dy
            if r2 - r <= dist2 <= r2 + r:
                canvas.set_px(x, y, color)


def draw_ellipse(canvas, cx, cy, rx, ry, color):
    if rx <= 0 or ry <= 0:
        return
    rx2 = rx * rx
    ry2 = ry * ry
    rxy2 = rx2 * ry2
    for y in range(cy - ry, cy + ry + 1):
        dy = y - cy
        dy2 = dy * dy
        for x in range(cx - rx, cx + rx + 1):
            dx = x - cx
            if dx * dx * ry2 + dy2 * rx2 <= rxy2:
                canvas.set_px(x, y, color)


def pick_ramp_color(ramp, dx_norm, dy_norm, highlight=True):
    if dy_norm < -0.55:
        return ramp["highlight"] if highlight else ramp["light"]
    if dy_norm > 0.6:
        return ramp["dark"]
    if dx_norm < -0.55:
        return ramp["light"]
    if dx_norm > 0.55:
        return ramp["dark"]
    return ramp["base"]


def fill_ellipse_shaded(canvas, cx, cy, rx, ry, ramp, highlight=True):
    if rx <= 0 or ry <= 0:
        return
    rx2 = rx * rx
    ry2 = ry * ry
    rxy2 = rx2 * ry2
    for y in range(cy - ry, cy + ry + 1):
        dy = y - cy
        dy2 = dy * dy
        for x in range(cx - rx, cx + rx + 1):
            dx = x - cx
            if dx * dx * ry2 + dy2 * rx2 <= rxy2:
                dx_norm = dx / rx
                dy_norm = dy / ry
                color = pick_ramp_color(ramp, dx_norm, dy_norm, highlight)
                canvas.set_px(x, y, color)


def fill_tapered_rect_shaded(canvas, cx, top, bottom, top_w, bottom_w, ramp):
    if bottom <= top or top_w <= 0 or bottom_w <= 0:
        return
    height = bottom - top
    for y in range(top, bottom):
        t = (y - top) / max(1, height - 1)
        width = int(top_w + (bottom_w - top_w) * t)
        half = width // 2
        for x in range(cx - half, cx + half + 1):
            dx_norm = 0.0 if half == 0 else (x - cx) / half
            dy_norm = 0.0 if height == 0 else (y - top) / height - 0.5
            color = pick_ramp_color(ramp, dx_norm, dy_norm, True)
            canvas.set_px(x, y, color)


def fill_rect_shaded(canvas, x, y, w, h, ramp, vertical=True, horizontal=True):
    if w <= 0 or h <= 0:
        return
    for iy in range(y, y + h):
        for ix in range(x, x + w):
            dx_norm = 0.0
            dy_norm = 0.0
            if horizontal:
                dx_norm = (ix - (x + w / 2)) / max(1.0, w / 2)
            if vertical:
                dy_norm = (iy - (y + h / 2)) / max(1.0, h / 2)
            color = pick_ramp_color(ramp, dx_norm, dy_norm, True)
            canvas.set_px(ix, iy, color)


def apply_outline_thick(canvas, outline_color, thickness):
    for _ in range(thickness):
        apply_outline(canvas, outline_color)


def apply_hatching(canvas, step, color, strength=0.6):
    w, h = canvas.width, canvas.height
    out = bytearray(canvas.pixels)
    for y in range(h):
        for x in range(w):
            if (x + y) % step != 0:
                continue
            idx = (y * w + x) * 4
            a = out[idx + 3]
            if a == 0:
                continue
            current = (out[idx], out[idx + 1], out[idx + 2])
            mixed = blend(current, color[:3], strength)
            out[idx] = mixed[0]
            out[idx + 1] = mixed[1]
            out[idx + 2] = mixed[2]
    canvas.pixels = out


def fill_ellipse_ink(canvas, cx, cy, rx, ry, ramp, hatch_step):
    if rx <= 0 or ry <= 0:
        return
    rx2 = rx * rx
    ry2 = ry * ry
    rxy2 = rx2 * ry2
    for y in range(cy - ry, cy + ry + 1):
        dy = y - cy
        dy2 = dy * dy
        for x in range(cx - rx, cx + rx + 1):
            dx = x - cx
            if dx * dx * ry2 + dy2 * rx2 > rxy2:
                continue
            dx_norm = dx / rx
            dy_norm = dy / ry
            shadow = dy_norm > 0.35 or dx_norm > 0.45
            deep_shadow = dy_norm > 0.6 or dx_norm > 0.65
            if dy_norm < -0.6 or dx_norm < -0.6:
                color = ramp["light"]
            else:
                color = ramp["base"]
            if shadow:
                color = ramp["dark"]
            if deep_shadow:
                color = ramp["darker"]
            if dy_norm < -0.75 and dx_norm < -0.75:
                color = ramp["highlight"]
            if shadow and (x + y) % hatch_step == 0:
                color = ramp["darker"]
            if deep_shadow and (x - y) % (hatch_step + 2) == 0:
                color = ramp["darker"]
            canvas.set_px(x, y, color)


def fill_tapered_rect_ink(canvas, cx, top, bottom, top_w, bottom_w, ramp, hatch_step):
    if bottom <= top or top_w <= 0 or bottom_w <= 0:
        return
    height = bottom - top
    for y in range(top, bottom):
        t = (y - top) / max(1, height - 1)
        width = int(top_w + (bottom_w - top_w) * t)
        half = width // 2
        for x in range(cx - half, cx + half + 1):
            dx_norm = 0.0 if half == 0 else (x - cx) / half
            dy_norm = 0.0 if height == 0 else (y - top) / height - 0.5
            shadow = dy_norm > 0.2 or dx_norm > 0.55
            deep_shadow = dy_norm > 0.45 or dx_norm > 0.75
            color = ramp["base"]
            if dy_norm < -0.4:
                color = ramp["light"]
            if shadow:
                color = ramp["dark"]
            if deep_shadow:
                color = ramp["darker"]
            if shadow and (x + y) % hatch_step == 0:
                color = ramp["darker"]
            if deep_shadow and (x - y) % (hatch_step + 2) == 0:
                color = ramp["darker"]
            canvas.set_px(x, y, color)


def fill_rect_ink(canvas, x, y, w, h, ramp, hatch_step, vertical=True, horizontal=True):
    if w <= 0 or h <= 0:
        return
    for iy in range(y, y + h):
        for ix in range(x, x + w):
            dx_norm = 0.0
            dy_norm = 0.0
            if horizontal:
                dx_norm = (ix - (x + w / 2)) / max(1.0, w / 2)
            if vertical:
                dy_norm = (iy - (y + h / 2)) / max(1.0, h / 2)
            shadow = dy_norm > 0.2 or dx_norm > 0.5
            deep_shadow = dy_norm > 0.45 or dx_norm > 0.75
            color = ramp["base"]
            if dy_norm < -0.4:
                color = ramp["light"]
            if shadow:
                color = ramp["dark"]
            if deep_shadow:
                color = ramp["darker"]
            if shadow and (ix + iy) % hatch_step == 0:
                color = ramp["darker"]
            if deep_shadow and (ix - iy) % (hatch_step + 2) == 0:
                color = ramp["darker"]
            canvas.set_px(ix, iy, color)


def silhouette_for_style(style_key):
    silhouette = {
        "head_scale": 1.0,
        "torso_len": 1.0,
        "shoulder": 1.0,
        "waist": 1.0,
        "hip": 1.0,
        "leg_len": 1.0,
        "leg_w": 1.0,
        "arm_len": 1.0,
        "arm_w": 1.0,
    }
    if style_key == "style02_flat":
        silhouette.update(
            {
                "head_scale": 1.3,
                "torso_len": 0.85,
                "shoulder": 0.95,
                "waist": 0.95,
                "hip": 1.0,
                "leg_len": 0.75,
                "leg_w": 1.0,
                "arm_len": 0.75,
                "arm_w": 1.0,
            }
        )
    elif style_key == "style03_noir":
        silhouette.update(
            {
                "head_scale": 0.9,
                "torso_len": 1.15,
                "shoulder": 0.9,
                "waist": 0.85,
                "hip": 0.9,
                "leg_len": 1.1,
                "leg_w": 0.9,
                "arm_len": 1.0,
                "arm_w": 0.85,
            }
        )
    elif style_key == "style04_pastel":
        silhouette.update(
            {
                "head_scale": 1.15,
                "torso_len": 0.95,
                "shoulder": 1.1,
                "waist": 1.15,
                "hip": 1.15,
                "leg_len": 0.85,
                "leg_w": 1.1,
                "arm_len": 0.9,
                "arm_w": 1.1,
            }
        )
    elif style_key == "style05_warm":
        silhouette.update(
            {
                "head_scale": 1.0,
                "torso_len": 1.05,
                "shoulder": 1.1,
                "waist": 1.0,
                "hip": 1.0,
                "leg_len": 1.0,
                "leg_w": 1.0,
                "arm_len": 1.0,
                "arm_w": 1.0,
            }
        )
    elif style_key == "style06_cool":
        silhouette.update(
            {
                "head_scale": 1.0,
                "torso_len": 1.0,
                "shoulder": 1.0,
                "waist": 1.0,
                "hip": 1.0,
                "leg_len": 1.05,
                "leg_w": 0.95,
                "arm_len": 1.0,
                "arm_w": 0.9,
            }
        )
    elif style_key == "style07_neon":
        silhouette.update(
            {
                "head_scale": 1.05,
                "torso_len": 1.0,
                "shoulder": 1.2,
                "waist": 0.9,
                "hip": 1.0,
                "leg_len": 1.0,
                "leg_w": 1.0,
                "arm_len": 1.0,
                "arm_w": 1.1,
            }
        )
    elif style_key == "style08_comic":
        silhouette.update(
            {
                "head_scale": 1.1,
                "torso_len": 1.0,
                "shoulder": 1.3,
                "waist": 0.85,
                "hip": 1.0,
                "leg_len": 1.0,
                "leg_w": 1.15,
                "arm_len": 1.05,
                "arm_w": 1.2,
            }
        )
    elif style_key == "style09_dither":
        silhouette.update(
            {
                "head_scale": 0.95,
                "torso_len": 1.05,
                "shoulder": 1.05,
                "waist": 0.95,
                "hip": 1.05,
                "leg_len": 1.0,
                "leg_w": 1.0,
                "arm_len": 1.0,
                "arm_w": 1.0,
            }
        )
    elif style_key == "style01_classic":
        silhouette.update(
            {
                "head_scale": 1.2,
                "torso_len": 1.0,
                "shoulder": 1.0,
                "waist": 1.0,
                "hip": 1.0,
                "leg_len": 1.0,
                "leg_w": 1.0,
                "arm_len": 1.0,
                "arm_w": 1.0,
            }
        )
    elif style_key == "style10_painted":
        silhouette.update(
            {
                "head_scale": 1.2,
                "torso_len": 1.0,
                "shoulder": 1.05,
                "waist": 1.0,
                "hip": 1.0,
                "leg_len": 1.0,
                "leg_w": 1.0,
                "arm_len": 1.0,
                "arm_w": 1.0,
            }
        )
    elif style_key == "style11_illustrated":
        silhouette.update(
            {
                "head_scale": 0.9,
                "torso_len": 1.2,
                "shoulder": 1.0,
                "waist": 0.9,
                "hip": 0.9,
                "leg_len": 1.2,
                "leg_w": 0.9,
                "arm_len": 1.1,
                "arm_w": 0.9,
            }
        )
    return silhouette


def accessory_for_style(style_key):
    head = "none"
    body = "none"
    if style_key == "style02_flat":
        head = "cap"
        body = "tunic_short"
    elif style_key == "style03_noir":
        head = "hat"
        body = "coat"
    elif style_key == "style04_pastel":
        head = "hood"
        body = "poncho"
    elif style_key == "style05_warm":
        body = "cape"
    elif style_key == "style06_cool":
        head = "visor"
        body = "scarf"
    elif style_key == "style07_neon":
        head = "helmet"
        body = "pads"
    elif style_key == "style08_comic":
        head = "mask"
        body = "emblem"
    elif style_key == "style09_dither":
        head = "hood"
        body = "armor"
    elif style_key == "style10_painted":
        body = "mantle"
    elif style_key == "style11_illustrated":
        head = "circlet"
        body = "robe"
    return {"head": head, "body": body}


def draw_head_accessory_top(canvas, accessory, ramps, accent, cx, head_top, head_radius):
    if accessory == "hat":
        color = ramps["tunic"]["dark"]
        brim_y = head_top + 1
        canvas.rect(cx - head_radius - 1, brim_y, head_radius * 2 + 3, 2, color)
        canvas.rect(cx - head_radius + 1, head_top - 2, head_radius * 2 - 1, 3, color)
    elif accessory == "cap":
        color = ramps["tunic"]["dark"]
        brim_y = head_top + 1
        canvas.rect(cx - head_radius, brim_y, head_radius * 2 + 1, 1, color)
        canvas.rect(cx - head_radius + 1, head_top - 1, head_radius * 2 - 1, 2, color)
    elif accessory == "hood":
        color = ramps["tunic"]["dark"]
        for y in range(head_top - 1, head_top + head_radius * 2 + 2):
            canvas.set_px(cx - head_radius - 1, y, color)
            canvas.set_px(cx + head_radius + 1, y, color)
        for x in range(cx - head_radius - 1, cx + head_radius + 2):
            canvas.set_px(x, head_top - 1, color)
    elif accessory == "helmet":
        color = ramps["tunic"]["dark"]
        for y in range(head_top - 1, head_top + head_radius):
            for x in range(cx - head_radius, cx + head_radius + 1):
                canvas.set_px(x, y, color)
        canvas.set_px(cx, head_top - 2, accent)
    elif accessory == "circlet":
        color = accent
        circlet_y = head_top + 1
        for x in range(cx - head_radius + 1, cx + head_radius):
            canvas.set_px(x, circlet_y, color)


def draw_head_accessory_face(canvas, accessory, accent, cx, eye_y):
    if accessory == "visor":
        for x in range(cx - 3, cx + 4):
            canvas.set_px(x, eye_y, accent)
    elif accessory == "mask":
        for x in range(cx - 3, cx + 4):
            canvas.set_px(x, eye_y + 2, accent)


def draw_head(canvas, spec, style, ramps, cx, head_top, head_radius):
    skin = ramps["skin"]
    hair = ramps["hair"]
    eyes = apply_style_color(rgba(spec.eyes), style)
    head_center_y = head_top + head_radius
    accessories = accessory_for_style(style.key)
    accent = apply_style_color(rgba(spec.accent), style)
    face_style = getattr(spec, "face_style", "classic")
    human_face = face_style == "human"
    for y in range(head_top - head_radius, head_top + head_radius + 1):
        for x in range(cx - head_radius - 1, cx + head_radius + 2):
            dx = x - cx
            dy = y - head_center_y
            if dx * dx + dy * dy <= head_radius * head_radius:
                color = skin["base"]
                if style.shading:
                    if x <= cx - 1 and y <= head_center_y - 1:
                        color = skin["light"]
                    elif x >= cx + 1 and y >= head_center_y + 1:
                        color = skin["dark"]
                canvas.set_px(x, y, color)

    # Ears
    if spec.ear in ("pointed", "long"):
        ear_color = hair["base"] if spec.ear == "long" else skin["base"]
        ear_dx = head_radius + 1
        ear_y = head_center_y
        canvas.set_px(cx - ear_dx, ear_y, ear_color)
        canvas.set_px(cx + ear_dx, ear_y, ear_color)
        if spec.ear == "pointed":
            canvas.set_px(cx - ear_dx - 1, ear_y - 1, ear_color)
            canvas.set_px(cx + ear_dx + 1, ear_y - 1, ear_color)
        if spec.ear == "long":
            canvas.set_px(cx - ear_dx - 1, ear_y, ear_color)
            canvas.set_px(cx + ear_dx + 1, ear_y, ear_color)
            canvas.set_px(cx - ear_dx - 1, ear_y + 1, ear_color)
            canvas.set_px(cx + ear_dx + 1, ear_y + 1, ear_color)

    # Hair
    for y in range(head_top - head_radius, head_top + 1):
        for x in range(cx - head_radius - 1, cx + head_radius + 2):
            dx = x - cx
            dy = y - (head_top + head_radius)
            if dx * dx + dy * dy <= head_radius * head_radius:
                if spec.hair_style in ("short", "braided", "cropped"):
                    if y <= head_center_y - 2:
                        color = hair["base"]
                        if style.shading and x <= cx - 1:
                            color = hair["light"]
                        canvas.set_px(x, y, color)
                elif spec.hair_style == "long":
                    if y <= head_center_y - 1 or abs(x - cx) >= head_radius - 1:
                        color = hair["base"]
                        if style.shading and x <= cx - 1:
                            color = hair["light"]
                        canvas.set_px(x, y, color)
                else:
                    if y <= head_center_y - 2:
                        color = hair["base"]
                        if style.shading and (x + y) % 2 == 0:
                            color = hair["light"]
                        canvas.set_px(x, y, color)

    draw_head_accessory_top(
        canvas,
        accessories["head"],
        ramps,
        accent,
        cx,
        head_top,
        head_radius,
    )

    # Eyes
    eye_offset = 1 if human_face else 2
    if human_face and head_radius >= 6:
        eye_offset = 2
    eye_y = head_center_y - 1 if human_face else head_center_y
    canvas.set_px(cx - eye_offset, eye_y, eyes)
    canvas.set_px(cx + eye_offset, eye_y, eyes)

    # Brows
    if spec.brow == "heavy":
        brow_color = hair["dark"]
        canvas.set_px(cx - 2, eye_y - 1, brow_color)
        canvas.set_px(cx + 2, eye_y - 1, brow_color)
    elif spec.brow == "soft" and style.detail >= 3:
        canvas.set_px(cx - 2, eye_y - 1, hair["base"])
        canvas.set_px(cx + 2, eye_y - 1, hair["base"])

    # Nose
    nose_y = eye_y + 1
    if spec.nose == "bulbous":
        canvas.set_px(cx, nose_y, skin["dark"])
        canvas.set_px(cx - 1, nose_y, skin["dark"])
    elif spec.nose == "prominent":
        canvas.set_px(cx, nose_y, skin["dark"])
    else:
        nose_color = skin["light"] if human_face and style.shading else skin["base"]
        canvas.set_px(cx, nose_y, nose_color)

    # Mouth
    mouth_color = skin["darker"] if style.detail >= 3 else skin["dark"]
    if human_face:
        mouth_y = eye_y + 2
        canvas.set_px(cx - 1, mouth_y, mouth_color)
        canvas.set_px(cx, mouth_y, mouth_color)
        if head_radius >= 5:
            canvas.set_px(cx + 1, mouth_y, mouth_color)
    else:
        mouth_y = eye_y + 3
        canvas.set_px(cx, mouth_y, mouth_color)

    draw_head_accessory_face(canvas, accessories["head"], accent, cx, eye_y)

    # Freckles
    if spec.freckles and style.detail >= 2:
        freckle = skin["darker"]
        canvas.set_px(cx - 3, eye_y + 1, freckle)
        canvas.set_px(cx + 3, eye_y + 1, freckle)

    # Rugged jaw
    if spec.rugged and style.detail >= 3:
        jaw_y = mouth_y + (1 if human_face else 0)
        jaw_dx = 2 if human_face else 3
        canvas.set_px(cx - jaw_dx, jaw_y, skin["dark"])
        canvas.set_px(cx + jaw_dx, jaw_y, skin["dark"])

    # Tattoo
    if spec.tattoo and style.detail >= 4:
        tattoo_color = apply_style_color(rgba(spec.tattoo), style)
        canvas.set_px(cx - 1, eye_y + 2, tattoo_color)
        canvas.set_px(cx, eye_y + 2, tattoo_color)
        canvas.set_px(cx + 1, eye_y + 2, tattoo_color)


def draw_body(canvas, spec, style, ramps, cx, torso_top, torso_bottom, silhouette):
    tunic = ramps["tunic"]
    pants = ramps["pants"]
    boots = ramps["boots"]
    skin = ramps["skin"]
    accent = apply_style_color(rgba(spec.accent), style)
    accessories = accessory_for_style(style.key)

    torso_h = torso_bottom - torso_top
    shoulder_w = max(6, min(14, int(8 * spec.build * silhouette["shoulder"])))
    waist_w = max(4, min(12, int(6 * spec.build * silhouette["waist"])))
    hip_w = max(5, min(12, int(7 * spec.build * silhouette["hip"])))

    for y in range(torso_top, torso_bottom):
        t = (y - torso_top) / max(1, torso_h - 1)
        width = int(shoulder_w + (waist_w - shoulder_w) * t)
        half = width // 2
        for x in range(cx - half, cx + half + 1):
            color = tunic["base"]
            if style.shading:
                if x <= cx - half + 1:
                    color = tunic["light"]
                elif x >= cx + half - 1:
                    color = tunic["dark"]
                if y >= torso_bottom - 2:
                    color = tunic["dark"]
            canvas.set_px(x, y, color)

    if accessories["body"] in ("cape", "poncho", "mantle"):
        if accessories["body"] == "cape":
            cape_x = cx - shoulder_w // 2 - 3
            for y in range(torso_top, SPRITE_H - 2):
                width = 3 if y < torso_bottom else 4
                canvas.rect(cape_x, y, width, 1, tunic["dark"])
        elif accessories["body"] == "poncho":
            poncho_top = torso_top + 1
            poncho_bottom = torso_top + max(3, torso_h // 2)
            for y in range(poncho_top, poncho_bottom):
                t = (y - poncho_top) / max(1, poncho_bottom - poncho_top)
                width = int((shoulder_w + 4) - t * 3)
                half = width // 2
                canvas.rect(cx - half, y, width + 1, 1, tunic["light"])
        elif accessories["body"] == "mantle":
            mantle_y = torso_top + 1
            canvas.rect(
                cx - shoulder_w // 2 - 1,
                mantle_y,
                shoulder_w + 3,
                2,
                tunic["dark"],
            )

    if style.detail >= 3:
        belt_y = torso_bottom - 2
        canvas.rect(cx - waist_w // 2, belt_y, waist_w + 1, 1, accent)
        if style.detail >= 4:
            canvas.set_px(cx, belt_y, rgba(blend(spec.accent, (255, 255, 255), 0.2)))
    if accessories["body"] == "tunic_short":
        hem_y = torso_bottom - 1
        canvas.rect(cx - waist_w // 2, hem_y, waist_w + 1, 1, accent)
    elif accessories["body"] == "scarf":
        scarf_y = torso_top + 2
        canvas.rect(cx - waist_w // 2, scarf_y, waist_w + 1, 1, accent)
    elif accessories["body"] == "pads":
        pad_w = max(2, shoulder_w // 4)
        canvas.rect(cx - shoulder_w // 2 - 1, torso_top + 1, pad_w, 2, accent)
        canvas.rect(
            cx + shoulder_w // 2 - pad_w + 1,
            torso_top + 1,
            pad_w,
            2,
            accent,
        )
    elif accessories["body"] == "emblem":
        emblem_y = torso_top + max(2, torso_h // 2)
        canvas.set_px(cx, emblem_y, accent)
        canvas.set_px(cx - 1, emblem_y, accent)
        canvas.set_px(cx + 1, emblem_y, accent)
        canvas.set_px(cx, emblem_y - 1, accent)
        canvas.set_px(cx, emblem_y + 1, accent)
    elif accessories["body"] == "armor":
        plate_y = torso_top + 2
        canvas.rect(cx - waist_w // 2, plate_y, waist_w + 1, 1, tunic["dark"])
        canvas.rect(
            cx - waist_w // 2,
            plate_y + 3,
            waist_w + 1,
            1,
            tunic["light"],
        )

    # Arms
    arm_len = max(5, int((torso_h - 2) * silhouette["arm_len"]))
    arm_offset = 1 + (1 if silhouette["arm_w"] > 1.1 else 0)
    arm_thick = 2 if silhouette["arm_w"] > 1.1 else 1
    arm_x_left = cx - shoulder_w // 2 - arm_offset
    arm_x_right = cx + shoulder_w // 2 + arm_offset
    sleeve_len = max(2, arm_len // 2)
    for i in range(arm_len):
        color = tunic["dark"] if i < sleeve_len else skin["base"]
        if style.shading and i >= sleeve_len:
            color = skin["light"] if i == sleeve_len else skin["base"]
        for t in range(arm_thick):
            canvas.set_px(arm_x_left - t, torso_top + 1 + i, color)
            canvas.set_px(arm_x_right + t, torso_top + 1 + i, color)

    # Legs
    leg_top = torso_bottom - 1
    max_leg = SPRITE_H - 3
    leg_bottom = leg_top + int((max_leg - leg_top) * silhouette["leg_len"])
    leg_bottom = max(leg_top + 4, min(max_leg, leg_bottom))
    leg_w = max(2, int(hip_w // 2 * silhouette["leg_w"]))
    for y in range(leg_top, leg_bottom + 1):
        for x in range(cx - leg_w - 1, cx):
            color = pants["base"]
            if style.shading and x <= cx - leg_w - 1:
                color = pants["light"]
            if style.shading and y >= leg_bottom - 1:
                color = pants["dark"]
            canvas.set_px(x, y, color)
        for x in range(cx + 1, cx + leg_w + 2):
            color = pants["base"]
            if style.shading and x >= cx + leg_w + 1:
                color = pants["dark"]
            if style.shading and y >= leg_bottom - 1:
                color = pants["dark"]
            canvas.set_px(x, y, color)

    # Boots
    boot_y = leg_bottom
    canvas.rect(cx - leg_w - 1, boot_y, leg_w + 1, 2, boots["base"])
    canvas.rect(cx + 1, boot_y, leg_w + 1, 2, boots["base"])
    if style.shading:
        canvas.set_px(cx - leg_w - 1, boot_y + 1, boots["dark"])
        canvas.set_px(cx + leg_w + 1, boot_y + 1, boots["dark"])

    if accessories["body"] in ("coat", "robe"):
        if accessories["body"] == "coat":
            panel_w = max(2, waist_w // 2)
            for y in range(leg_top, leg_bottom + 1):
                canvas.rect(cx - panel_w - 1, y, panel_w, 1, tunic["dark"])
                canvas.rect(cx + 1, y, panel_w, 1, tunic["dark"])
        else:
            for y in range(leg_top, leg_bottom + 1):
                canvas.rect(cx - hip_w // 2, y, hip_w + 1, 1, tunic["base"])
            canvas.set_px(cx, leg_bottom - 1, tunic["dark"])

    # Shoulder trim
    if style.detail >= 4:
        trim_color = rgba(blend(spec.accent, (255, 255, 255), 0.3))
        canvas.set_px(cx - shoulder_w // 2, torso_top + 1, trim_color)
        canvas.set_px(cx + shoulder_w // 2, torso_top + 1, trim_color)


def render_character_variant(spec, style):
    mode = getattr(style, "render_mode", "classic")
    if mode == "ink":
        return render_character_ink(spec, style)
    if mode == "highres":
        return render_character_highres(spec, style)
    if mode == "profile":
        return render_character_profile(spec, style)
    if mode == "chibi":
        return render_character_chibi(spec, style)
    if mode == "blocky":
        return render_character_blocky(spec, style)
    if mode == "silhouette":
        return render_character_silhouette(spec, style)
    if mode == "lineart":
        return render_character_lineart(spec, style)
    return render_character_blocky(spec, style)


def render_character_ink(spec, style):
    s = HIGHRES_SCALE
    width = SPRITE_W * s
    height = SPRITE_H * s
    canvas = Canvas(width, height)
    ramps = {
        "skin": make_ramp(spec.skin, style),
        "hair": make_ramp(spec.hair, style),
        "tunic": make_ramp(spec.tunic, style),
        "pants": make_ramp(spec.pants, style),
        "boots": make_ramp(spec.boots, style),
    }
    skin = ramps["skin"]
    hair = ramps["hair"]
    tunic = ramps["tunic"]
    pants = ramps["pants"]
    boots = ramps["boots"]
    accent = apply_style_color(rgba(spec.accent), style)
    eyes = apply_style_color(rgba(spec.eyes), style)
    hatch_step = max(4, s // 2)
    skin_hatch = hatch_step + 3
    cloth_hatch = hatch_step
    pants_hatch = hatch_step + 1

    cx = width // 2
    head_rx = int(5.2 * s)
    head_ry = int(6.4 * s)
    head_top = int(2.6 * s)
    head_cy = head_top + head_ry
    fill_ellipse_ink(canvas, cx, head_cy, head_rx, head_ry, skin, skin_hatch)

    hair_cap_bottom = head_cy - int(head_ry * 0.25)
    for y in range(head_top - int(0.6 * s), hair_cap_bottom):
        for x in range(cx - head_rx, cx + head_rx + 1):
            dx = x - cx
            dy = y - head_cy
            if dx * dx * head_ry * head_ry + dy * dy * head_rx * head_rx <= head_rx * head_rx * head_ry * head_ry:
                hair_shade = hair["base"]
                if dy < -head_ry * 0.4:
                    hair_shade = hair["light"]
                if (x + y) % (hatch_step + 1) == 0:
                    hair_shade = hair["dark"]
                canvas.set_px(x, y, hair_shade)

    if spec.hair_style == "long":
        lock_w = int(3.0 * s)
        lock_h = int(7.0 * s)
        canvas.rect(cx - head_rx - lock_w + 2, hair_cap_bottom, lock_w, lock_h, hair["base"])
        canvas.rect(cx + head_rx - 1, hair_cap_bottom, lock_w, lock_h, hair["base"])
        for y in range(hair_cap_bottom, hair_cap_bottom + lock_h, 2):
            canvas.set_px(cx - head_rx - 1, y, hair["dark"])
            canvas.set_px(cx + head_rx + lock_w - 2, y, hair["dark"])
    elif spec.hair_style == "wild":
        spike_h = int(3.5 * s)
        for i in range(-6, 7):
            x0 = cx + int(i * 1.1 * s)
            canvas.line(x0, head_top + int(0.6 * s), x0, head_top - spike_h, hair["dark"])

    # Ears
    ear_y = head_cy - int(0.2 * s)
    ear_w = int(1.4 * s)
    if spec.ear == "pointed":
        for i in range(int(2.2 * s)):
            canvas.set_px(cx - head_rx - i, ear_y - i, skin["dark"])
            canvas.set_px(cx - head_rx - i, ear_y + i, skin["dark"])
    elif spec.ear == "long":
        canvas.rect(cx - head_rx - ear_w, ear_y, ear_w, int(3.4 * s), skin["dark"])
    else:
        canvas.rect(cx - head_rx - ear_w, ear_y, ear_w, ear_w, skin["dark"])

    eye_y = head_cy - int(head_ry * 0.18)
    eye_w = int(2.2 * s)
    eye_h = max(2, int(0.8 * s))
    left_eye_x = cx - int(3.6 * s)
    right_eye_x = cx + int(1.6 * s)
    canvas.rect(left_eye_x, eye_y, eye_w, eye_h, skin["light"])
    canvas.rect(right_eye_x, eye_y, eye_w, eye_h, skin["light"])
    canvas.rect(left_eye_x + eye_w // 3, eye_y + 1, eye_w // 2, eye_h - 1, eyes)
    canvas.rect(right_eye_x + eye_w // 3, eye_y + 1, eye_w // 2, eye_h - 1, eyes)
    canvas.set_px(left_eye_x + eye_w // 2, eye_y + 1, skin["dark"])
    canvas.set_px(right_eye_x + eye_w // 2, eye_y + 1, skin["dark"])

    brow_y = eye_y - int(0.8 * s)
    brow_w = int(2.8 * s)
    brow_h = int(0.6 * s)
    brow_color = hair["dark"] if spec.brow == "heavy" else hair["base"]
    canvas.rect(left_eye_x, brow_y, brow_w, brow_h, brow_color)
    canvas.rect(right_eye_x, brow_y, brow_w, brow_h, brow_color)
    if spec.brow == "heavy":
        canvas.rect(left_eye_x, brow_y + 1, brow_w, brow_h, brow_color)
        canvas.rect(right_eye_x, brow_y + 1, brow_w, brow_h, brow_color)

    nose_x = cx + int(0.7 * s)
    if spec.nose == "prominent":
        canvas.rect(nose_x, eye_y + int(0.6 * s), int(0.6 * s), int(2.4 * s), skin["dark"])
        canvas.rect(nose_x + 1, eye_y + int(0.9 * s), int(0.3 * s), int(1.6 * s), skin["light"])
    elif spec.nose == "bulbous":
        draw_circle(canvas, nose_x, eye_y + int(1.5 * s), int(0.9 * s), skin["dark"])
    else:
        canvas.set_px(nose_x, eye_y + int(1.0 * s), skin["dark"])
    mouth_y = eye_y + int(2.8 * s)
    canvas.rect(cx - int(1.0 * s), mouth_y, int(2.0 * s), int(0.5 * s), skin["darker"])

    # Cheek and lip highlights for softer planes.
    canvas.rect(cx - int(2.6 * s), eye_y + int(1.2 * s), int(1.4 * s), int(0.6 * s), skin["highlight"])
    canvas.rect(cx + int(1.4 * s), eye_y + int(1.2 * s), int(1.0 * s), int(0.4 * s), skin["light"])
    canvas.rect(cx - int(0.6 * s), mouth_y - int(0.4 * s), int(1.2 * s), int(0.3 * s), skin["light"])

    # Neck
    neck_w = int(2.8 * s)
    neck_h = int(1.6 * s)
    neck_x = cx - neck_w // 2
    neck_y = head_cy + head_ry - int(0.2 * s)
    fill_rect_ink(canvas, neck_x, neck_y, neck_w, neck_h, skin, skin_hatch, True, True)

    if spec.freckles:
        freckle = skin["darker"]
        canvas.set_px(cx - int(2.0 * s), eye_y + int(1.4 * s), freckle)
        canvas.set_px(cx - int(1.3 * s), eye_y + int(1.6 * s), freckle)
        canvas.set_px(cx + int(1.8 * s), eye_y + int(1.5 * s), freckle)
    if spec.rugged:
        jaw_y = mouth_y + int(0.8 * s)
        canvas.rect(cx - int(1.6 * s), jaw_y, int(3.2 * s), int(0.6 * s), skin["dark"])

    if spec.tattoo and style.detail >= 3:
        tattoo_color = apply_style_color(rgba(spec.tattoo), style)
        canvas.rect(cx + int(1.5 * s), eye_y + int(1.8 * s), int(2.0 * s), int(0.8 * s), tattoo_color)

    torso_top = head_cy + head_ry - int(1.2 * s)
    torso_len = int(13 * s * spec.height)
    torso_bottom = min(height - int(7 * s), torso_top + torso_len)
    shoulder_w = int(11.5 * s * spec.build)
    waist_w = int(7.8 * s * spec.build)
    fill_tapered_rect_ink(canvas, cx, torso_top, torso_bottom, shoulder_w, waist_w, tunic, cloth_hatch)

    # Cloak / mantle
    cloak_top = torso_top - int(0.8 * s)
    cloak_bottom = torso_top + int(3.2 * s)
    cloak_w = int(shoulder_w * 1.15)
    cloak_base = shade(rgba(spec.tunic), 0.7)
    cloak_ramp = make_ramp(cloak_base, style)
    fill_tapered_rect_ink(
        canvas,
        cx,
        cloak_top,
        cloak_bottom,
        cloak_w,
        cloak_w - int(2.0 * s),
        cloak_ramp,
        cloth_hatch,
    )

    collar_y = torso_top + int(0.5 * s)
    canvas.rect(cx - int(3.4 * s), collar_y, int(6.8 * s), int(0.8 * s), tunic["dark"])
    canvas.rect(cx - int(3.0 * s), collar_y + int(0.2 * s), int(6.0 * s), int(0.3 * s), tunic["light"])

    belt_y = torso_bottom - int(1.6 * s)
    canvas.rect(cx - waist_w // 2, belt_y, waist_w + 1, int(0.8 * s), accent)
    buckle_w = int(1.2 * s)
    canvas.rect(cx - buckle_w // 2, belt_y, buckle_w, int(0.8 * s), tunic["dark"])
    # Belt loop and pouch
    loop_x = cx - int(2.2 * s)
    canvas.rect(loop_x, belt_y - int(0.6 * s), int(0.6 * s), int(1.6 * s), tunic["dark"])
    pouch_w = int(2.4 * s)
    pouch_h = int(2.0 * s)
    pouch_x = cx + int(2.4 * s)
    canvas.rect(pouch_x, belt_y - int(0.4 * s), pouch_w, pouch_h, tunic["darker"])
    canvas.rect(pouch_x, belt_y - int(0.4 * s), pouch_w, int(0.4 * s), tunic["dark"])

    if spec.rugged:
        shoulder_patch_w = int(2.8 * s)
        shoulder_patch_h = int(1.2 * s)
        canvas.rect(cx - shoulder_w // 2, torso_top + int(0.8 * s), shoulder_patch_w, shoulder_patch_h, accent)
        for i in range(int(3.0 * s)):
            canvas.set_px(cx - shoulder_w // 2 + i, torso_top + int(0.8 * s) + i, tunic["dark"])

    torso_h = torso_bottom - torso_top
    arm_len = int(torso_h * 0.75)
    sleeve_len = int(arm_len * 0.55)
    arm_w = int(2.8 * s)
    arm_x_left = cx - shoulder_w // 2 - arm_w
    arm_x_right = cx + shoulder_w // 2
    for i in range(arm_len):
        y = torso_top + int(0.7 * s) + i
        arm_color = tunic["dark"] if i < sleeve_len else skin["base"]
        canvas.rect(arm_x_left, y, arm_w, 1, arm_color)
        canvas.rect(arm_x_right, y, arm_w, 1, arm_color)
        if i == sleeve_len - int(0.2 * s):
            canvas.rect(arm_x_left, y, arm_w, int(0.3 * s), tunic["light"])
            canvas.rect(arm_x_right, y, arm_w, int(0.3 * s), tunic["light"])
    hand_h = int(1.0 * s)
    canvas.rect(arm_x_left, torso_top + int(0.7 * s) + arm_len, arm_w, hand_h, skin["light"])
    canvas.rect(arm_x_right, torso_top + int(0.7 * s) + arm_len, arm_w, hand_h, skin["light"])

    if spec.tattoo and style.detail >= 3:
        tattoo_color = apply_style_color(rgba(spec.tattoo), style)
        canvas.rect(arm_x_left + int(0.6 * s), torso_top + int(2.0 * s), int(1.6 * s), int(0.6 * s), tattoo_color)

    leg_top = torso_bottom - int(0.4 * s)
    leg_len = int(8.8 * s)
    leg_w = int(3.2 * s * spec.build)
    fill_rect_ink(canvas, cx - leg_w - 2, leg_top, leg_w, leg_len, pants, pants_hatch, True, True)
    fill_rect_ink(canvas, cx + 2, leg_top, leg_w, leg_len, pants, pants_hatch, True, True)

    knee_y = leg_top + int(3.8 * s)
    canvas.rect(cx - leg_w - 2, knee_y, leg_w, int(0.6 * s), pants["dark"])
    canvas.rect(cx + 2, knee_y, leg_w, int(0.6 * s), pants["dark"])

    boot_h = int(2.4 * s)
    sole_h = int(0.5 * s)
    fill_rect_ink(canvas, cx - leg_w - 2, leg_top + leg_len - 1, leg_w, boot_h, boots, pants_hatch, True, False)
    fill_rect_ink(canvas, cx + 2, leg_top + leg_len - 1, leg_w, boot_h, boots, pants_hatch, True, False)
    canvas.rect(cx - leg_w - 2, leg_top + leg_len - 1 + boot_h - sole_h, leg_w, sole_h, boots["dark"])
    canvas.rect(cx + 2, leg_top + leg_len - 1 + boot_h - sole_h, leg_w, sole_h, boots["dark"])
    cuff_h = int(0.6 * s)
    canvas.rect(cx - leg_w - 2, leg_top + leg_len - boot_h, leg_w, cuff_h, boots["dark"])
    canvas.rect(cx + 2, leg_top + leg_len - boot_h, leg_w, cuff_h, boots["dark"])

    apply_outline_thick(canvas, style.outline_color, max(2, s // 6))
    return canvas.width, canvas.height, canvas.pixels

def render_character_highres(spec, style):
    s = HIGHRES_SCALE
    width = SPRITE_W * s
    height = SPRITE_H * s
    canvas = Canvas(width, height)
    ramps = {
        "skin": make_ramp(spec.skin, style),
        "hair": make_ramp(spec.hair, style),
        "tunic": make_ramp(spec.tunic, style),
        "pants": make_ramp(spec.pants, style),
        "boots": make_ramp(spec.boots, style),
    }
    skin = ramps["skin"]
    hair = ramps["hair"]
    tunic = ramps["tunic"]
    pants = ramps["pants"]
    boots = ramps["boots"]
    accent = apply_style_color(rgba(spec.accent), style)
    eyes = apply_style_color(rgba(spec.eyes), style)

    cx = width // 2
    head_rx = int(5.0 * s)
    head_ry = int(6.2 * s)
    head_top = int(3.0 * s)
    head_cy = head_top + head_ry
    fill_ellipse_shaded(canvas, cx, head_cy, head_rx, head_ry, skin, True)

    # Hair cap and styles
    hair_cap_bottom = head_cy - int(head_ry * 0.3)
    if spec.hair_style == "cropped":
        hair_cap_bottom = head_cy - int(head_ry * 0.1)
    elif spec.hair_style == "short":
        hair_cap_bottom = head_cy - int(head_ry * 0.2)
    elif spec.hair_style == "long":
        hair_cap_bottom = head_cy + int(head_ry * 0.2)
    elif spec.hair_style == "wild":
        hair_cap_bottom = head_cy - int(head_ry * 0.15)

    for y in range(head_top - int(0.6 * s), hair_cap_bottom):
        for x in range(cx - head_rx, cx + head_rx + 1):
            dx = x - cx
            dy = y - head_cy
            if dx * dx * head_ry * head_ry + dy * dy * head_rx * head_rx <= head_rx * head_rx * head_ry * head_ry:
                shade = hair["base"]
                if dy < -head_ry * 0.4:
                    shade = hair["light"]
                elif dy > head_ry * 0.1:
                    shade = hair["dark"]
                canvas.set_px(x, y, shade)

    if spec.hair_style == "long":
        lock_w = int(2.6 * s)
        lock_h = int(6.0 * s)
        canvas.rect(cx - head_rx - lock_w + 2, hair_cap_bottom, lock_w, lock_h, hair["base"])
        canvas.rect(cx + head_rx - 1, hair_cap_bottom, lock_w, lock_h, hair["base"])
    elif spec.hair_style == "wild":
        spike_h = int(3.0 * s)
        for i in range(-5, 6):
            x0 = cx + int(i * 1.2 * s)
            canvas.line(x0, head_top + int(0.6 * s), x0, head_top - spike_h, hair["dark"])

    for x in range(cx - head_rx + 2, cx + head_rx - 1):
        canvas.set_px(x, head_top + int(1.0 * s), hair["light"])

    # Ears
    ear_y = head_cy - int(0.2 * s)
    ear_w = int(1.4 * s)
    if spec.ear == "pointed":
        for i in range(int(2.0 * s)):
            canvas.set_px(cx - head_rx - i, ear_y - i, skin["dark"])
            canvas.set_px(cx - head_rx - i, ear_y + i, skin["dark"])
    elif spec.ear == "long":
        canvas.rect(cx - head_rx - ear_w, ear_y, ear_w, int(3.0 * s), skin["dark"])
    else:
        canvas.rect(cx - head_rx - ear_w, ear_y, ear_w, ear_w, skin["dark"])

    # Brows + eyes
    eye_y = head_cy - int(head_ry * 0.18)
    eye_w = int(2.0 * s)
    eye_h = max(2, int(0.8 * s))
    left_eye_x = cx - int(3.4 * s)
    right_eye_x = cx + int(1.4 * s)
    canvas.rect(left_eye_x, eye_y, eye_w, eye_h, skin["light"])
    canvas.rect(right_eye_x, eye_y, eye_w, eye_h, skin["light"])
    canvas.rect(left_eye_x + eye_w // 3, eye_y + 1, eye_w // 2, eye_h - 1, eyes)
    canvas.rect(right_eye_x + eye_w // 3, eye_y + 1, eye_w // 2, eye_h - 1, eyes)
    canvas.set_px(left_eye_x + eye_w // 2, eye_y + 1, skin["dark"])
    canvas.set_px(right_eye_x + eye_w // 2, eye_y + 1, skin["dark"])
    brow_y = eye_y - int(0.8 * s)
    brow_w = int(2.6 * s)
    brow_h = int(0.6 * s)
    brow_color = hair["dark"] if spec.brow == "heavy" else hair["base"]
    canvas.rect(left_eye_x, brow_y, brow_w, brow_h, brow_color)
    canvas.rect(right_eye_x, brow_y, brow_w, brow_h, brow_color)
    if spec.brow == "heavy":
        canvas.rect(left_eye_x, brow_y + 1, brow_w, brow_h, brow_color)
        canvas.rect(right_eye_x, brow_y + 1, brow_w, brow_h, brow_color)

    # Nose + mouth
    nose_x = cx + int(0.7 * s)
    if spec.nose == "prominent":
        canvas.rect(nose_x, eye_y + int(0.6 * s), int(0.6 * s), int(2.2 * s), skin["dark"])
        canvas.rect(nose_x + 1, eye_y + int(0.8 * s), int(0.3 * s), int(1.6 * s), skin["light"])
    elif spec.nose == "bulbous":
        draw_circle(canvas, nose_x, eye_y + int(1.4 * s), int(0.9 * s), skin["dark"])
    else:
        canvas.set_px(nose_x, eye_y + int(1.0 * s), skin["dark"])
    mouth_y = eye_y + int(2.8 * s)
    canvas.rect(cx - int(1.0 * s), mouth_y, int(2.0 * s), int(0.5 * s), skin["darker"])

    # Freckles + rugged jaw
    if spec.freckles:
        freckle = skin["darker"]
        canvas.set_px(cx - int(2.0 * s), eye_y + int(1.4 * s), freckle)
        canvas.set_px(cx - int(1.3 * s), eye_y + int(1.6 * s), freckle)
        canvas.set_px(cx + int(1.8 * s), eye_y + int(1.5 * s), freckle)
    if spec.rugged:
        jaw_y = mouth_y + int(0.8 * s)
        canvas.rect(cx - int(1.6 * s), jaw_y, int(3.2 * s), int(0.6 * s), skin["dark"])

    if spec.tattoo and style.detail >= 3:
        tattoo_color = apply_style_color(rgba(spec.tattoo), style)
        canvas.rect(cx + int(1.5 * s), eye_y + int(1.8 * s), int(2.0 * s), int(0.8 * s), tattoo_color)

    # Torso
    torso_top = head_cy + head_ry - int(1.2 * s)
    torso_len = int(13 * s * spec.height)
    torso_bottom = min(height - int(7 * s), torso_top + torso_len)
    torso_h = torso_bottom - torso_top
    shoulder_w = int(11.0 * s * spec.build)
    waist_w = int(7.8 * s * spec.build)
    fill_tapered_rect_shaded(canvas, cx, torso_top, torso_bottom, shoulder_w, waist_w, tunic)

    collar_y = torso_top + int(0.6 * s)
    canvas.rect(cx - int(3.2 * s), collar_y, int(6.4 * s), int(0.7 * s), tunic["dark"])

    belt_y = torso_bottom - int(1.6 * s)
    canvas.rect(cx - waist_w // 2, belt_y, waist_w + 1, int(0.8 * s), accent)
    buckle_w = int(1.2 * s)
    canvas.rect(cx - buckle_w // 2, belt_y, buckle_w, int(0.8 * s), tunic["dark"])

    if spec.rugged:
        shoulder_patch_w = int(2.6 * s)
        shoulder_patch_h = int(1.2 * s)
        canvas.rect(cx - shoulder_w // 2, torso_top + int(0.8 * s), shoulder_patch_w, shoulder_patch_h, accent)

    # Arms
    arm_len = int(torso_h * 0.75)
    sleeve_len = int(arm_len * 0.55)
    arm_w = int(2.6 * s)
    arm_x_left = cx - shoulder_w // 2 - arm_w
    arm_x_right = cx + shoulder_w // 2
    for i in range(arm_len):
        y = torso_top + int(0.7 * s) + i
        if i < sleeve_len:
            arm_color = tunic["dark"]
        else:
            arm_color = skin["base"]
        canvas.rect(arm_x_left, y, arm_w, 1, arm_color)
        canvas.rect(arm_x_right, y, arm_w, 1, arm_color)
    hand_h = int(1.0 * s)
    canvas.rect(arm_x_left, torso_top + int(0.7 * s) + arm_len, arm_w, hand_h, skin["light"])
    canvas.rect(arm_x_right, torso_top + int(0.7 * s) + arm_len, arm_w, hand_h, skin["light"])

    if spec.tattoo and style.detail >= 3:
        tattoo_color = apply_style_color(rgba(spec.tattoo), style)
        canvas.rect(arm_x_left + int(0.6 * s), torso_top + int(2.0 * s), int(1.6 * s), int(0.6 * s), tattoo_color)

    # Legs
    leg_top = torso_bottom - int(0.4 * s)
    leg_len = int(8.5 * s)
    leg_w = int(3.2 * s * spec.build)
    for y in range(leg_top, leg_top + leg_len):
        for x in range(cx - leg_w - 2, cx - 2):
            color = pants["base"]
            if x <= cx - leg_w - 1:
                color = pants["light"]
            if y >= leg_top + leg_len - int(1.8 * s):
                color = pants["dark"]
            canvas.set_px(x, y, color)
        for x in range(cx + 2, cx + leg_w + 2):
            color = pants["base"]
            if x >= cx + leg_w + 1:
                color = pants["dark"]
            if y >= leg_top + leg_len - int(1.8 * s):
                color = pants["dark"]
            canvas.set_px(x, y, color)
    knee_y = leg_top + int(3.8 * s)
    canvas.rect(cx - leg_w - 2, knee_y, leg_w, int(0.6 * s), pants["dark"])
    canvas.rect(cx + 2, knee_y, leg_w, int(0.6 * s), pants["dark"])

    boot_h = int(2.4 * s)
    sole_h = int(0.5 * s)
    canvas.rect(cx - leg_w - 2, leg_top + leg_len - 1, leg_w, boot_h, boots["base"])
    canvas.rect(cx + 2, leg_top + leg_len - 1, leg_w, boot_h, boots["base"])
    canvas.rect(cx - leg_w - 2, leg_top + leg_len - 1 + boot_h - sole_h, leg_w, sole_h, boots["dark"])
    canvas.rect(cx + 2, leg_top + leg_len - 1 + boot_h - sole_h, leg_w, sole_h, boots["dark"])

    if style.outline:
        apply_outline(canvas, style.outline_color)
    return canvas.width, canvas.height, canvas.pixels


def render_character_profile(spec, style):
    canvas = Canvas(SPRITE_W, SPRITE_H)
    ramps = {
        "skin": make_ramp(spec.skin, style),
        "hair": make_ramp(spec.hair, style),
        "tunic": make_ramp(spec.tunic, style),
        "pants": make_ramp(spec.pants, style),
        "boots": make_ramp(spec.boots, style),
    }
    skin = ramps["skin"]["base"]
    hair = ramps["hair"]["base"]
    tunic = ramps["tunic"]["base"]
    pants = ramps["pants"]["base"]
    boots = ramps["boots"]["base"]
    accent = apply_style_color(rgba(spec.accent), style)
    eyes = apply_style_color(rgba(spec.eyes), style)

    head_r = 4
    head_cx = 6
    head_cy = 8
    draw_circle(canvas, head_cx, head_cy, head_r, skin)
    for y in range(head_cy - head_r, head_cy - head_r + 2):
        for x in range(head_cx - head_r, head_cx + head_r + 1):
            canvas.set_px(x, y, hair)
    canvas.set_px(head_cx + head_r - 1, head_cy, eyes)
    canvas.set_px(head_cx + head_r, head_cy + 1, skin)
    canvas.set_px(head_cx - head_r, head_cy + 1, skin)

    torso_x = head_cx + head_r - 1
    torso_y = head_cy + head_r - 1
    torso_w = 6
    torso_h = 9
    canvas.rect(torso_x, torso_y, torso_w, torso_h, tunic)
    canvas.rect(torso_x, torso_y + 4, torso_w, 1, accent)

    arm_y = torso_y + 3
    canvas.line(torso_x + 1, arm_y, torso_x + torso_w + 2, arm_y + 1, skin)

    leg_y = torso_y + torso_h - 1
    canvas.rect(torso_x + 1, leg_y, 2, 6, pants)
    canvas.rect(torso_x + 3, leg_y + 1, 2, 5, pants)
    canvas.rect(torso_x + 1, leg_y + 5, 2, 2, boots)
    canvas.rect(torso_x + 3, leg_y + 5, 2, 2, boots)

    if style.outline:
        apply_outline(canvas, style.outline_color)
    return canvas.width, canvas.height, canvas.pixels


def render_character_chibi(spec, style):
    canvas = Canvas(SPRITE_W, SPRITE_H)
    ramps = {
        "skin": make_ramp(spec.skin, style),
        "hair": make_ramp(spec.hair, style),
        "tunic": make_ramp(spec.tunic, style),
        "pants": make_ramp(spec.pants, style),
        "boots": make_ramp(spec.boots, style),
    }
    skin = ramps["skin"]["base"]
    hair = ramps["hair"]["base"]
    tunic = ramps["tunic"]["base"]
    pants = ramps["pants"]["base"]
    boots = ramps["boots"]["base"]
    eyes = apply_style_color(rgba(spec.eyes), style)

    head_r = 6
    head_cx = SPRITE_W // 2
    head_cy = 8
    draw_circle(canvas, head_cx, head_cy, head_r, skin)
    for y in range(head_cy - head_r, head_cy - head_r + 3):
        for x in range(head_cx - head_r, head_cx + head_r + 1):
            canvas.set_px(x, y, hair)
    canvas.set_px(head_cx - 2, head_cy, eyes)
    canvas.set_px(head_cx + 2, head_cy, eyes)

    body_w = 8
    body_h = 6
    body_x = head_cx - body_w // 2
    body_y = head_cy + head_r - 1
    canvas.rect(body_x, body_y, body_w, body_h, tunic)

    canvas.rect(body_x - 2, body_y + 2, 2, 3, skin)
    canvas.rect(body_x + body_w, body_y + 2, 2, 3, skin)

    leg_y = body_y + body_h
    canvas.rect(body_x + 1, leg_y, 3, 4, pants)
    canvas.rect(body_x + body_w - 4, leg_y, 3, 4, pants)
    canvas.rect(body_x + 1, leg_y + 3, 3, 2, boots)
    canvas.rect(body_x + body_w - 4, leg_y + 3, 3, 2, boots)

    if style.outline:
        apply_outline(canvas, style.outline_color)
    return canvas.width, canvas.height, canvas.pixels


def render_character_blocky(spec, style):
    canvas = Canvas(SPRITE_W, SPRITE_H)
    ramps = {
        "skin": make_ramp(spec.skin, style),
        "hair": make_ramp(spec.hair, style),
        "tunic": make_ramp(spec.tunic, style),
        "pants": make_ramp(spec.pants, style),
        "boots": make_ramp(spec.boots, style),
    }
    skin = ramps["skin"]["base"]
    hair = ramps["hair"]["base"]
    tunic = ramps["tunic"]["base"]
    pants = ramps["pants"]["base"]
    boots = ramps["boots"]["base"]

    head_w = 6
    head_h = 6
    head_x = SPRITE_W // 2 - head_w // 2
    head_y = 4
    canvas.rect(head_x, head_y, head_w, head_h, skin)
    canvas.rect(head_x, head_y, head_w, 2, hair)

    body_w = 10
    body_h = 10
    body_x = SPRITE_W // 2 - body_w // 2
    body_y = head_y + head_h + 1
    canvas.rect(body_x, body_y, body_w, body_h, tunic)

    canvas.rect(body_x - 2, body_y + 2, 2, 7, skin)
    canvas.rect(body_x + body_w, body_y + 2, 2, 7, skin)

    leg_y = body_y + body_h
    canvas.rect(body_x + 1, leg_y, 3, 6, pants)
    canvas.rect(body_x + body_w - 4, leg_y, 3, 6, pants)
    canvas.rect(body_x + 1, leg_y + 5, 3, 2, boots)
    canvas.rect(body_x + body_w - 4, leg_y + 5, 3, 2, boots)

    if style.outline:
        apply_outline(canvas, style.outline_color)
    return canvas.width, canvas.height, canvas.pixels


def render_character_silhouette(spec, style):
    canvas = Canvas(SPRITE_W, SPRITE_H)
    ramps = {
        "tunic": make_ramp(spec.tunic, style),
    }
    sil = ramps["tunic"]["darker"]
    head_r = 5
    head_cx = SPRITE_W // 2
    head_cy = 8
    draw_circle(canvas, head_cx, head_cy, head_r, sil)
    body_x = head_cx - 6
    body_y = head_cy + head_r - 1
    body_w = 12
    body_h = 14
    canvas.rect(body_x, body_y, body_w, body_h, sil)
    canvas.rect(body_x + 2, body_y + body_h, 3, 6, sil)
    canvas.rect(body_x + body_w - 5, body_y + body_h, 3, 6, sil)
    return canvas.width, canvas.height, canvas.pixels


def render_character_lineart(spec, style):
    canvas = Canvas(SPRITE_W, SPRITE_H)
    ink = apply_style_color(rgba(spec.accent), style)
    head_w = 8
    head_h = 8
    head_x = SPRITE_W // 2 - head_w // 2
    head_y = 4
    draw_rect_outline(canvas, head_x, head_y, head_w, head_h, ink)

    body_w = 10
    body_h = 10
    body_x = SPRITE_W // 2 - body_w // 2
    body_y = head_y + head_h
    draw_rect_outline(canvas, body_x, body_y, body_w, body_h, ink)

    arm_y = body_y + 3
    canvas.line(body_x - 2, arm_y, body_x, arm_y + 2, ink)
    canvas.line(body_x + body_w, arm_y + 2, body_x + body_w + 2, arm_y, ink)

    leg_y = body_y + body_h
    canvas.line(body_x + 2, leg_y, body_x + 2, leg_y + 6, ink)
    canvas.line(body_x + body_w - 3, leg_y, body_x + body_w - 3, leg_y + 6, ink)
    return canvas.width, canvas.height, canvas.pixels


def render_character_downed_variant(spec, style):
    mode = getattr(style, "render_mode", "classic")
    if mode == "ink":
        return render_character_ink_downed(spec, style)
    if mode == "highres":
        return render_character_highres_downed(spec, style)
    if mode == "profile":
        return render_character_profile_downed(spec, style)
    if mode == "chibi":
        return render_character_chibi_downed(spec, style)
    if mode == "blocky":
        return render_character_blocky_downed(spec, style)
    if mode == "silhouette":
        return render_character_silhouette_downed(spec, style)
    if mode == "lineart":
        return render_character_lineart_downed(spec, style)
    return render_character_blocky_downed(spec, style)


def render_character_ink_downed(spec, style):
    s = HIGHRES_SCALE
    width = SPRITE_W * s
    height = SPRITE_H * s
    canvas = Canvas(width, height)
    ramps = {
        "skin": make_ramp(spec.skin, style),
        "hair": make_ramp(spec.hair, style),
        "tunic": make_ramp(spec.tunic, style),
        "pants": make_ramp(spec.pants, style),
        "boots": make_ramp(spec.boots, style),
    }
    skin = ramps["skin"]
    hair = ramps["hair"]
    tunic = ramps["tunic"]
    pants = ramps["pants"]
    boots = ramps["boots"]
    accent = apply_style_color(rgba(spec.accent), style)
    eyes = apply_style_color(rgba(spec.eyes), style)
    hatch_step = max(4, s // 2)
    skin_hatch = hatch_step + 3
    cloth_hatch = hatch_step
    pants_hatch = hatch_step + 1

    base_y = height - int(5 * s)
    head_rx = int(5.0 * s)
    head_ry = int(6.0 * s)
    head_cx = int(6.6 * s)
    head_cy = base_y - head_ry
    fill_ellipse_ink(canvas, head_cx, head_cy, head_rx, head_ry, skin, skin_hatch)

    hair_cap_bottom = head_cy - int(head_ry * 0.3)
    for y in range(head_cy - head_ry, hair_cap_bottom):
        for x in range(head_cx - head_rx, head_cx + head_rx + 1):
            dx = x - head_cx
            dy = y - head_cy
            if dx * dx * head_ry * head_ry + dy * dy * head_rx * head_rx <= head_rx * head_rx * head_ry * head_ry:
                canvas.set_px(x, y, hair["base"])
    canvas.rect(head_cx + head_rx - int(2.0 * s), head_cy, int(1.6 * s), int(1.2 * s), eyes)

    body_x = head_cx + head_rx - int(1.0 * s)
    body_y = base_y - int(5.8 * s)
    body_w = int(16 * s)
    body_h = int(5.2 * s)
    fill_rect_ink(canvas, body_x, body_y, body_w, body_h, tunic, cloth_hatch, True, True)
    canvas.rect(body_x + int(2 * s), body_y + int(1.6 * s), int(4 * s), int(0.8 * s), accent)

    arm_x = body_x + int(3.2 * s)
    arm_y = body_y + int(0.4 * s)
    fill_rect_ink(canvas, arm_x, arm_y, int(3.2 * s), int(1.2 * s), skin, skin_hatch, False, False)

    leg_x = body_x + body_w - int(3.2 * s)
    fill_rect_ink(canvas, leg_x, body_y + int(1.0 * s), int(5.2 * s), int(2.6 * s), pants, pants_hatch, True, True)
    canvas.rect(leg_x + int(3.2 * s), body_y + int(2.0 * s), int(3.0 * s), int(1.8 * s), boots["dark"])

    apply_outline_thick(canvas, style.outline_color, max(2, s // 6))
    return canvas.width, canvas.height, canvas.pixels


def render_character_highres_downed(spec, style):
    s = HIGHRES_SCALE
    width = SPRITE_W * s
    height = SPRITE_H * s
    canvas = Canvas(width, height)
    ramps = {
        "skin": make_ramp(spec.skin, style),
        "hair": make_ramp(spec.hair, style),
        "tunic": make_ramp(spec.tunic, style),
        "pants": make_ramp(spec.pants, style),
        "boots": make_ramp(spec.boots, style),
    }
    skin = ramps["skin"]
    hair = ramps["hair"]
    tunic = ramps["tunic"]
    pants = ramps["pants"]
    boots = ramps["boots"]
    accent = apply_style_color(rgba(spec.accent), style)
    eyes = apply_style_color(rgba(spec.eyes), style)

    base_y = height - int(5 * s)
    head_rx = int(5.0 * s)
    head_ry = int(6.0 * s)
    head_cx = int(6.5 * s)
    head_cy = base_y - head_ry
    fill_ellipse_shaded(canvas, head_cx, head_cy, head_rx, head_ry, skin, True)

    hair_cap_bottom = head_cy - int(head_ry * 0.3)
    for y in range(head_cy - head_ry, hair_cap_bottom):
        for x in range(head_cx - head_rx, head_cx + head_rx + 1):
            dx = x - head_cx
            dy = y - head_cy
            if dx * dx * head_ry * head_ry + dy * dy * head_rx * head_rx <= head_rx * head_rx * head_ry * head_ry:
                canvas.set_px(x, y, hair["base"])
    canvas.rect(head_cx + head_rx - int(2.0 * s), head_cy, int(1.6 * s), int(1.2 * s), eyes)

    body_x = head_cx + head_rx - int(1.0 * s)
    body_y = base_y - int(5.8 * s)
    body_w = int(16 * s)
    body_h = int(5.2 * s)
    fill_rect_shaded(canvas, body_x, body_y, body_w, body_h, tunic, True, True)
    canvas.rect(body_x + int(2 * s), body_y + int(1.6 * s), int(4 * s), int(0.8 * s), accent)

    arm_x = body_x + int(3.2 * s)
    arm_y = body_y + int(0.4 * s)
    fill_rect_shaded(canvas, arm_x, arm_y, int(3.2 * s), int(1.2 * s), skin, False, False)

    leg_x = body_x + body_w - int(3.2 * s)
    fill_rect_shaded(canvas, leg_x, body_y + int(1.0 * s), int(5.2 * s), int(2.6 * s), pants, True, True)
    canvas.rect(leg_x + int(3.2 * s), body_y + int(2.0 * s), int(3.0 * s), int(1.8 * s), boots["dark"])

    if style.outline:
        apply_outline(canvas, style.outline_color)
    return canvas.width, canvas.height, canvas.pixels

def render_character_profile_downed(spec, style):
    canvas = Canvas(SPRITE_W, SPRITE_H)
    ramps = {
        "skin": make_ramp(spec.skin, style),
        "hair": make_ramp(spec.hair, style),
        "tunic": make_ramp(spec.tunic, style),
        "pants": make_ramp(spec.pants, style),
        "boots": make_ramp(spec.boots, style),
    }
    skin = ramps["skin"]["base"]
    hair = ramps["hair"]["base"]
    tunic = ramps["tunic"]["base"]
    pants = ramps["pants"]["base"]
    boots = ramps["boots"]["base"]
    accent = apply_style_color(rgba(spec.accent), style)
    eyes = apply_style_color(rgba(spec.eyes), style)

    base_y = SPRITE_H - 4
    head_r = 4
    head_cx = 6
    head_cy = base_y - 4
    draw_circle(canvas, head_cx, head_cy, head_r, skin)
    for y in range(head_cy - head_r, head_cy - head_r + 2):
        for x in range(head_cx - head_r, head_cx + head_r + 1):
            canvas.set_px(x, y, hair)
    canvas.set_px(head_cx + head_r - 1, head_cy, eyes)

    body_x = head_cx + head_r - 1
    body_y = base_y - 6
    body_w = 10
    body_h = 5
    canvas.rect(body_x, body_y, body_w, body_h, tunic)
    canvas.rect(body_x + 2, body_y + 2, 4, 1, accent)

    leg_x = body_x + body_w - 2
    canvas.rect(leg_x, body_y + 1, 5, 3, pants)
    canvas.rect(leg_x + 3, body_y + 2, 3, 2, boots)

    if style.outline:
        apply_outline(canvas, style.outline_color)
    return canvas.width, canvas.height, canvas.pixels


def render_character_chibi_downed(spec, style):
    canvas = Canvas(SPRITE_W, SPRITE_H)
    ramps = {
        "skin": make_ramp(spec.skin, style),
        "hair": make_ramp(spec.hair, style),
        "tunic": make_ramp(spec.tunic, style),
        "pants": make_ramp(spec.pants, style),
        "boots": make_ramp(spec.boots, style),
    }
    skin = ramps["skin"]["base"]
    hair = ramps["hair"]["base"]
    tunic = ramps["tunic"]["base"]
    pants = ramps["pants"]["base"]
    boots = ramps["boots"]["base"]
    eyes = apply_style_color(rgba(spec.eyes), style)

    base_y = SPRITE_H - 5
    head_r = 6
    head_cx = 7
    head_cy = base_y - 6
    draw_circle(canvas, head_cx, head_cy, head_r, skin)
    for y in range(head_cy - head_r, head_cy - head_r + 3):
        for x in range(head_cx - head_r, head_cx + head_r + 1):
            canvas.set_px(x, y, hair)
    canvas.set_px(head_cx - 2, head_cy, eyes)

    body_x = head_cx + head_r - 1
    body_y = base_y - 4
    body_w = 8
    body_h = 4
    canvas.rect(body_x, body_y, body_w, body_h, tunic)
    canvas.rect(body_x + 1, body_y + 2, 3, 2, pants)
    canvas.rect(body_x + 4, body_y + 2, 3, 2, pants)
    canvas.rect(body_x + 1, body_y + 3, 3, 2, boots)
    canvas.rect(body_x + 4, body_y + 3, 3, 2, boots)
    return canvas.width, canvas.height, canvas.pixels


def render_character_blocky_downed(spec, style):
    canvas = Canvas(SPRITE_W, SPRITE_H)
    ramps = {
        "skin": make_ramp(spec.skin, style),
        "hair": make_ramp(spec.hair, style),
        "tunic": make_ramp(spec.tunic, style),
        "pants": make_ramp(spec.pants, style),
        "boots": make_ramp(spec.boots, style),
    }
    skin = ramps["skin"]["base"]
    hair = ramps["hair"]["base"]
    tunic = ramps["tunic"]["base"]
    pants = ramps["pants"]["base"]
    boots = ramps["boots"]["base"]

    base_y = SPRITE_H - 4
    head_w = 6
    head_h = 6
    head_x = 3
    head_y = base_y - head_h
    canvas.rect(head_x, head_y, head_w, head_h, skin)
    canvas.rect(head_x, head_y, head_w, 2, hair)

    body_x = head_x + head_w - 1
    body_y = base_y - 5
    body_w = 12
    body_h = 5
    canvas.rect(body_x, body_y, body_w, body_h, tunic)

    leg_x = body_x + body_w - 3
    canvas.rect(leg_x, body_y + 1, 5, 3, pants)
    canvas.rect(leg_x + 3, body_y + 2, 3, 2, boots)
    return canvas.width, canvas.height, canvas.pixels


def render_character_silhouette_downed(spec, style):
    canvas = Canvas(SPRITE_W, SPRITE_H)
    ramps = {"tunic": make_ramp(spec.tunic, style)}
    sil = ramps["tunic"]["darker"]
    base_y = SPRITE_H - 4
    head_r = 5
    head_cx = 6
    head_cy = base_y - 5
    draw_circle(canvas, head_cx, head_cy, head_r, sil)
    body_x = head_cx + head_r - 1
    body_y = base_y - 5
    canvas.rect(body_x, body_y, 12, 5, sil)
    canvas.rect(body_x + 8, body_y + 1, 5, 3, sil)
    return canvas.width, canvas.height, canvas.pixels


def render_character_lineart_downed(spec, style):
    canvas = Canvas(SPRITE_W, SPRITE_H)
    ink = apply_style_color(rgba(spec.accent), style)
    base_y = SPRITE_H - 4
    head_w = 8
    head_h = 8
    head_x = 3
    head_y = base_y - head_h
    draw_rect_outline(canvas, head_x, head_y, head_w, head_h, ink)
    body_x = head_x + head_w - 1
    body_y = base_y - 5
    draw_rect_outline(canvas, body_x, body_y, 10, 5, ink)
    canvas.line(body_x + 7, body_y + 1, body_x + 12, body_y + 3, ink)
    return canvas.width, canvas.height, canvas.pixels


def render_character(spec, style):
    if getattr(style, "render_mode", "classic") != "classic":
        return render_character_variant(spec, style)
    canvas = Canvas(SPRITE_W, SPRITE_H)
    ramps = {
        "skin": make_ramp(spec.skin, style),
        "hair": make_ramp(spec.hair, style),
        "tunic": make_ramp(spec.tunic, style),
        "pants": make_ramp(spec.pants, style),
        "boots": make_ramp(spec.boots, style),
    }

    cx = SPRITE_W // 2
    silhouette = silhouette_for_style(style.key)
    head_radius = max(3, min(6, int(round(4 * silhouette["head_scale"]))))
    head_top = 4
    height_offset = int((spec.height - 1.0) * 3)
    head_top = max(2, head_top - height_offset // 2 - (head_radius - 4))
    torso_top = head_top + head_radius + 2
    torso_len = int((10 + height_offset) * silhouette["torso_len"])
    torso_bottom = torso_top + torso_len
    torso_bottom = min(SPRITE_H - 6, torso_bottom)

    draw_head(canvas, spec, style, ramps, cx, head_top, head_radius)
    draw_body(canvas, spec, style, ramps, cx, torso_top, torso_bottom, silhouette)

    if style.outline:
        apply_outline(canvas, style.outline_color)

    return canvas.width, canvas.height, canvas.pixels


def render_character_downed(spec, style):
    if getattr(style, "render_mode", "classic") != "classic":
        return render_character_downed_variant(spec, style)
    canvas = Canvas(SPRITE_W, SPRITE_H)
    ramps = {
        "skin": make_ramp(spec.skin, style),
        "hair": make_ramp(spec.hair, style),
        "tunic": make_ramp(spec.tunic, style),
        "pants": make_ramp(spec.pants, style),
        "boots": make_ramp(spec.boots, style),
    }

    silhouette = silhouette_for_style(style.key)
    accessories = accessory_for_style(style.key)

    base_y = SPRITE_H - 3
    head_radius = max(3, min(6, int(round(4 * silhouette["head_scale"]))))
    head_top = base_y - 11
    head_cx = max(2, int(3 * silhouette["head_scale"]))

    draw_head(canvas, spec, style, ramps, head_cx, head_top, head_radius)

    torso_h = 6 + max(0, int((spec.build - 1.0) * 2))
    torso_len = int((13 + max(0, int((spec.build - 1.0) * 2))) * silhouette["torso_len"])
    torso_x = head_cx + head_radius - 1
    torso_len = min(torso_len, SPRITE_W - torso_x - 1)
    torso_y = base_y - torso_h + 1
    tunic = ramps["tunic"]
    pants = ramps["pants"]
    boots = ramps["boots"]
    accent = apply_style_color(rgba(spec.accent), style)

    for y in range(torso_y, torso_y + torso_h):
        for x in range(torso_x, torso_x + torso_len):
            color = tunic["base"]
            if style.shading:
                if y >= torso_y + torso_h - 1:
                    color = tunic["dark"]
                elif y == torso_y:
                    color = tunic["light"]
            canvas.set_px(x, y, color)

    arm_y = torso_y + 2
    arm_color = ramps["skin"]["base"]
    canvas.set_px(torso_x + 2, arm_y, arm_color)
    canvas.set_px(torso_x + 3, arm_y, arm_color)
    canvas.set_px(torso_x + 4, arm_y + 1, arm_color)
    canvas.set_px(torso_x + 5, arm_y + 1, arm_color)

    leg_len = max(5, int(7 * silhouette["leg_len"]))
    leg_h = 4
    leg_x = torso_x + torso_len - 3
    leg_y = torso_y + torso_h - leg_h + 1
    for y in range(leg_y, leg_y + leg_h):
        for x in range(leg_x, leg_x + leg_len):
            color = pants["base"]
            if style.shading and y == leg_y + leg_h - 1:
                color = pants["dark"]
            canvas.set_px(x, y, color)
    for y in range(leg_y - 1, leg_y + leg_h - 1):
        for x in range(leg_x - 2, leg_x + leg_len - 2):
            color = pants["darker"] if style.shading else pants["base"]
            canvas.set_px(x, y, color)

    boot_y = leg_y + leg_h - 1
    canvas.rect(leg_x + leg_len - 3, boot_y, 4, 2, boots["base"])
    if style.shading:
        canvas.set_px(leg_x + leg_len - 3, boot_y + 1, boots["dark"])

    if accessories["body"] == "scarf":
        scarf_y = torso_y + 1
        canvas.rect(torso_x + 1, scarf_y, min(5, torso_len - 2), 1, accent)
    elif accessories["body"] == "armor":
        canvas.rect(torso_x + 1, torso_y + 1, min(6, torso_len - 2), 1, tunic["dark"])
    elif accessories["body"] == "mantle":
        canvas.rect(torso_x, torso_y, min(6, torso_len), 2, tunic["dark"])
    elif accessories["body"] in ("coat", "robe"):
        fill_color = tunic["dark"] if accessories["body"] == "coat" else tunic["base"]
        for y in range(leg_y - 1, leg_y + leg_h + 1):
            canvas.rect(torso_x + 1, y, max(4, torso_len - 4), 1, fill_color)
    elif accessories["body"] == "cape":
        cape_x = torso_x - 2
        for y in range(torso_y, leg_y + leg_h + 1):
            canvas.rect(cape_x, y, 2, 1, tunic["dark"])
    elif accessories["body"] == "poncho":
        for y in range(torso_y, torso_y + 3):
            canvas.rect(torso_x, y, min(8, torso_len), 1, tunic["light"])
    elif accessories["body"] == "pads":
        canvas.rect(torso_x + 1, torso_y, 2, 2, accent)
    elif accessories["body"] == "emblem":
        emblem_x = torso_x + min(4, torso_len - 3)
        canvas.set_px(emblem_x, torso_y + 2, accent)
        canvas.set_px(emblem_x - 1, torso_y + 2, accent)
        canvas.set_px(emblem_x + 1, torso_y + 2, accent)

    if style.detail >= 3:
        hair = ramps["hair"]
        canvas.set_px(head_cx - 2, head_top + head_radius + 1, hair["dark"])
        canvas.set_px(head_cx - 1, head_top + head_radius + 2, hair["dark"])

    # Ground shadow
    shadow_color = apply_style_color((12, 12, 16, 120), style)
    canvas.rect(head_cx - 2, base_y + 1, torso_len + 8, 2, shadow_color)

    if style.outline:
        apply_outline(canvas, style.outline_color)

    return canvas.width, canvas.height, canvas.pixels


def scale_pixels(width, height, pixels, scale):
    if scale == 1:
        return width, height, pixels
    new_w = width * scale
    new_h = height * scale
    new_pixels = bytearray(new_w * new_h * 4)
    for y in range(height):
        for x in range(width):
            idx = (y * width + x) * 4
            px = pixels[idx : idx + 4]
            for dy in range(scale):
                for dx in range(scale):
                    nidx = ((y * scale + dy) * new_w + (x * scale + dx)) * 4
                    new_pixels[nidx : nidx + 4] = px
    return new_w, new_h, new_pixels


def scale_pixels_smooth(width, height, pixels, scale):
    if scale == 1:
        return width, height, pixels
    new_w = width * scale
    new_h = height * scale
    new_pixels = bytearray(new_w * new_h * 4)
    for y in range(new_h):
        src_y = y / scale
        y0 = int(math.floor(src_y))
        y1 = min(height - 1, y0 + 1)
        ty = src_y - y0
        for x in range(new_w):
            src_x = x / scale
            x0 = int(math.floor(src_x))
            x1 = min(width - 1, x0 + 1)
            tx = src_x - x0
            idx00 = (y0 * width + x0) * 4
            idx10 = (y0 * width + x1) * 4
            idx01 = (y1 * width + x0) * 4
            idx11 = (y1 * width + x1) * 4
            out = [0, 0, 0, 0]
            for c in range(4):
                v00 = pixels[idx00 + c]
                v10 = pixels[idx10 + c]
                v01 = pixels[idx01 + c]
                v11 = pixels[idx11 + c]
                v0 = v00 * (1 - tx) + v10 * tx
                v1 = v01 * (1 - tx) + v11 * tx
                out[c] = clamp(v0 * (1 - ty) + v1 * ty)
            out_idx = (y * new_w + x) * 4
            new_pixels[out_idx : out_idx + 4] = bytes(out)
    return new_w, new_h, new_pixels


def stylize_palette(colors, style):
    return {key: apply_style_color(value, style) for key, value in colors.items()}


def apply_postprocess_pixels(width, height, pixels, style):
    if style.dither or style.noise:
        pixels = apply_dither_and_noise(width, height, pixels, style)
    if style.blur_radius > 0:
        pixels = blur_pixels(width, height, pixels, style.blur_radius)
    if style.edge_soften:
        pixels = soften_edges(width, height, pixels, style.edge_soften_strength)
    if style.gradient_top != 1.0 or style.gradient_bottom != 1.0:
        pixels = apply_vertical_gradient(
            width, height, pixels, style.gradient_top, style.gradient_bottom
        )
    return width, height, pixels


def apply_dither_and_noise(width, height, pixels, style):
    out = bytearray(pixels)
    for y in range(height):
        for x in range(width):
            idx = (y * width + x) * 4
            if out[idx + 3] == 0:
                continue
            delta = 0
            if style.dither:
                if (x + y) % 2 == 0:
                    delta += style.dither_strength
                else:
                    delta -= style.dither_strength
            if style.noise and style.noise_strength > 0:
                seed = (x * 374761393 + y * 668265263 + len(style.key) * 1013904223) & 0xFFFFFFFF
                noise = ((seed >> 13) ^ seed) & 0xFF
                delta += int((noise / 255.0 - 0.5) * 2 * style.noise_strength)
            if delta != 0:
                out[idx] = clamp(out[idx] + delta)
                out[idx + 1] = clamp(out[idx + 1] + delta)
                out[idx + 2] = clamp(out[idx + 2] + delta)
    return out


def soften_edges(width, height, pixels, strength):
    if strength <= 0:
        return pixels
    out = bytearray(pixels)
    for y in range(height):
        for x in range(width):
            idx = (y * width + x) * 4
            alpha = pixels[idx + 3]
            if alpha == 0:
                continue
            total_a = 0
            total_r = 0
            total_g = 0
            total_b = 0
            count = 0
            for ky in (-1, 0, 1):
                ny = y + ky
                if ny < 0 or ny >= height:
                    continue
                for kx in (-1, 0, 1):
                    nx = x + kx
                    if nx < 0 or nx >= width:
                        continue
                    nidx = (ny * width + nx) * 4
                    na = pixels[nidx + 3]
                    if na == 0:
                        continue
                    total_a += na
                    total_r += pixels[nidx]
                    total_g += pixels[nidx + 1]
                    total_b += pixels[nidx + 2]
                    count += 1
            if count == 0:
                continue
            avg_a = total_a / count
            avg_r = total_r / count
            avg_g = total_g / count
            avg_b = total_b / count
            new_a = clamp(alpha * (1 - strength) + avg_a * strength)
            mix = strength * 0.5
            out[idx] = clamp(pixels[idx] * (1 - mix) + avg_r * mix)
            out[idx + 1] = clamp(pixels[idx + 1] * (1 - mix) + avg_g * mix)
            out[idx + 2] = clamp(pixels[idx + 2] * (1 - mix) + avg_b * mix)
            out[idx + 3] = new_a
    return out


def apply_vertical_gradient(width, height, pixels, top_factor, bottom_factor):
    if height <= 1:
        return pixels
    out = bytearray(pixels)
    for y in range(height):
        t = y / (height - 1)
        factor = top_factor + (bottom_factor - top_factor) * t
        for x in range(width):
            idx = (y * width + x) * 4
            if out[idx + 3] == 0:
                continue
            out[idx] = clamp(out[idx] * factor)
            out[idx + 1] = clamp(out[idx + 1] * factor)
            out[idx + 2] = clamp(out[idx + 2] * factor)
    return out


def blur_pixels(width, height, pixels, radius):
    if radius <= 0:
        return pixels
    out = bytearray(len(pixels))
    for y in range(height):
        for x in range(width):
            total = [0, 0, 0, 0]
            count = 0
            for ky in range(-radius, radius + 1):
                ny = y + ky
                if ny < 0 or ny >= height:
                    continue
                for kx in range(-radius, radius + 1):
                    nx = x + kx
                    if nx < 0 or nx >= width:
                        continue
                    idx = (ny * width + nx) * 4
                    for c in range(4):
                        total[c] += pixels[idx + c]
                    count += 1
            out_idx = (y * width + x) * 4
            if count:
                out[out_idx] = clamp(total[0] / count)
                out[out_idx + 1] = clamp(total[1] / count)
                out[out_idx + 2] = clamp(total[2] / count)
                out[out_idx + 3] = clamp(total[3] / count)
    return out


def write_png(path, width, height, pixels):
    raw = b"".join(
        b"\x00" + pixels[y * width * 4 : (y + 1) * width * 4] for y in range(height)
    )
    compressed = zlib.compress(raw, 9)

    def chunk(tag, data):
        return (
            struct.pack(">I", len(data))
            + tag
            + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        )

    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n")
        ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
        f.write(chunk(b"IHDR", ihdr))
        f.write(chunk(b"IDAT", compressed))
        f.write(chunk(b"IEND", b""))


# Weapon rendering

def draw_sword(canvas, style, colors, profile, long=False):
    w, h = canvas.width, canvas.height
    mid = h // 2
    blade_len = w - 4
    if long:
        blade_len = w - 3
    blade_thick = max(1, profile["blade"])
    blade_half = blade_thick // 2
    guard_w = max(2, profile["guard"])
    guard_h = max(1, blade_thick + 1)
    hilt_thick = max(1, profile["shaft"])
    # Hilt
    canvas.rect(0, mid - hilt_thick // 2, 2, hilt_thick, colors["leather"])
    canvas.rect(2, mid - guard_h // 2, 1, guard_h, colors["metal_dark"])
    canvas.rect(1, mid - guard_h // 2, guard_w, 1, colors["metal_dark"])
    # Blade
    for x in range(3, blade_len):
        for dy in range(-blade_half, blade_thick - blade_half):
            y = mid + dy
            canvas.set_px(x, y, colors["metal_light"])
            if style.detail >= 3 and x % 3 == 0 and dy == -blade_half:
                canvas.set_px(x, y, colors["metal_high"])
    canvas.set_px(blade_len, mid, colors["metal_high"])
    if profile["spike"]:
        canvas.set_px(blade_len - 1, mid - 1, colors["metal_high"])


def draw_axe(canvas, style, colors, profile):
    w, h = canvas.width, canvas.height
    mid = h // 2
    shaft_end = w - 5
    shaft_thick = max(1, profile["shaft"])
    head_size = max(3, profile["head"])
    canvas.rect(0, mid - shaft_thick // 2, shaft_end, shaft_thick, colors["wood"])
    canvas.rect(shaft_end, mid - head_size // 2, 2, head_size, colors["metal_dark"])
    canvas.rect(shaft_end + 2, mid - head_size, 3, head_size * 2, colors["metal_light"])
    if style.detail >= 3:
        canvas.set_px(shaft_end + 3, mid - 1, colors["metal_high"])


def draw_blunt(canvas, style, colors, profile):
    w, h = canvas.width, canvas.height
    mid = h // 2
    shaft_end = w - 4
    shaft_thick = max(1, profile["shaft"])
    head_size = max(3, profile["head"])
    canvas.rect(0, mid - shaft_thick // 2, shaft_end, shaft_thick, colors["wood"])
    canvas.rect(shaft_end, mid - head_size // 2, 3, head_size, colors["metal_dark"])
    canvas.rect(shaft_end + 1, mid - head_size // 2, 2, head_size, colors["metal_light"])
    if style.detail >= 3:
        canvas.set_px(shaft_end + 2, mid, colors["metal_high"])


def draw_bow(canvas, style, colors, profile):
    w, h = canvas.width, canvas.height
    mid = h // 2
    grip_x = 1
    string_x = 0
    max_offset = max(1, w - 4)
    thickness = max(1, profile["shaft"])
    curve = profile["curve"]
    for y in range(h):
        t = abs(y - mid) / max(1, mid)
        dist = (1.0 - t) ** 0.7
        x = grip_x + int(dist * max_offset * curve)
        for dx in range(thickness):
            canvas.set_px(min(x + dx, w - 1), y, colors["wood"])
        if style.detail >= 3 and y % 2 == 0:
            canvas.set_px(min(x + thickness, w - 1), y, colors["metal_high"])
    canvas.line(string_x, 0, string_x, h - 1, colors["string"])
    grip_h = max(3, thickness + 1)
    canvas.rect(grip_x, mid - grip_h // 2, 2, grip_h, colors["leather"])
    if style.detail >= 3:
        canvas.set_px(grip_x + 1, mid, colors["metal_high"])


def draw_crossbow(canvas, style, colors, profile):
    w, h = canvas.width, canvas.height
    mid = h // 2
    stock = w - 4
    shaft_thick = max(1, profile["shaft"])
    arm_thick = max(1, profile["head"] - 1)
    arm_height = max(4, h - 2)
    arm_x = stock - 1
    canvas.rect(1, mid - shaft_thick // 2, stock, shaft_thick, colors["wood"])
    canvas.rect(0, mid - shaft_thick // 2 - 1, 2, shaft_thick + 2, colors["wood"])
    canvas.rect(arm_x, mid - arm_height // 2, arm_thick, arm_height, colors["metal_dark"])
    canvas.line(arm_x, mid, w - 1, mid, colors["string"])
    canvas.rect(arm_x + 1, mid - 2, 2, 4, colors["metal_light"])
    if style.detail >= 3:
        canvas.set_px(arm_x, mid - arm_height // 2, colors["metal_high"])
        canvas.set_px(arm_x, mid + arm_height // 2 - 1, colors["metal_high"])
        canvas.line(2, mid - shaft_thick // 2 - 1, stock - 2, mid - shaft_thick // 2 - 1, colors["metal_high"])


def draw_double(canvas, style, colors, profile):
    w, h = canvas.width, canvas.height
    mid = h // 2
    shaft_thick = max(1, profile["shaft"])
    head_size = max(3, profile["head"])
    canvas.rect(2, mid - shaft_thick // 2, w - 4, shaft_thick, colors["wood"])
    canvas.rect(0, mid - head_size // 2, 2, head_size, colors["metal_light"])
    canvas.rect(w - 2, mid - head_size // 2, 2, head_size, colors["metal_light"])
    if style.detail >= 3:
        canvas.set_px(1, mid - 1, colors["metal_high"])
        canvas.set_px(w - 2, mid - 1, colors["metal_high"])


def draw_net(canvas, style, colors, profile):
    w, h = canvas.width, canvas.height
    canvas.rect(1, 1, w - 2, h - 2, (0, 0, 0, 0))
    canvas.line(1, 1, w - 2, h - 2, colors["string"])
    canvas.line(1, h - 2, w - 2, 1, colors["string"])
    canvas.line(1, h // 2, w - 2, h // 2, colors["string"])
    canvas.line(w // 2, 1, w // 2, h - 2, colors["string"])
    handle_thick = max(1, profile["shaft"])
    canvas.rect(0, h // 2 - handle_thick // 2, 2, handle_thick, colors["wood"])


def draw_whip(canvas, style, colors, profile):
    w, h = canvas.width, canvas.height
    mid = h // 2
    handle_thick = max(1, profile["shaft"])
    canvas.rect(0, mid - handle_thick // 2, 2, handle_thick, colors["leather"])
    for x in range(2, w):
        y = mid + ((x // 2) % 2)
        if y >= h:
            y = h - 1
        canvas.set_px(x, y, colors["string"])


def draw_polearm(canvas, style, colors, profile):
    w, h = canvas.width, canvas.height
    mid = h // 2
    shaft_thick = max(1, profile["shaft"])
    head_width = 7
    shaft = max(1, w - head_width)
    canvas.rect(0, mid - shaft_thick // 2, shaft, shaft_thick, colors["wood"])

    # Butt cap
    canvas.set_px(0, mid, colors["metal_dark"])
    if shaft_thick > 1:
        canvas.set_px(0, mid - 1, colors["metal_dark"])
    if style.detail >= 3:
        canvas.set_px(1, mid, colors["metal_high"])

    # Socket
    socket_x = shaft
    socket_thick = max(2, shaft_thick + 1)
    canvas.rect(
        socket_x, mid - socket_thick // 2, 2, socket_thick, colors["metal_dark"]
    )

    # Spear tip
    tip_x = w - 1
    canvas.set_px(tip_x, mid, colors["metal_high"])
    canvas.set_px(tip_x - 1, mid, colors["metal_light"])
    canvas.set_px(tip_x - 2, mid, colors["metal_light"])
    canvas.set_px(tip_x - 1, mid - 1, colors["metal_light"])
    canvas.set_px(tip_x - 1, mid + 1, colors["metal_light"])
    canvas.set_px(tip_x - 2, mid - 1, colors["metal_dark"])
    canvas.set_px(tip_x - 2, mid + 1, colors["metal_dark"])

    # Axe blade on top
    blade_x = socket_x + 2
    blade_top = max(0, mid - 2)
    blade_bottom = min(h - 1, mid)
    for y in range(blade_top, blade_bottom + 1):
        canvas.set_px(blade_x, y, colors["metal_dark"])
        canvas.set_px(blade_x + 1, y, colors["metal_light"])
        if y == blade_top or y == blade_bottom:
            canvas.set_px(blade_x + 2, y, colors["metal_dark"])
        else:
            canvas.set_px(blade_x + 2, y, colors["metal_high"])

    # Small spike below the socket
    spike_y = min(h - 1, mid + 2)
    canvas.set_px(blade_x + 1, spike_y, colors["metal_light"])
    if style.detail >= 3:
        canvas.set_px(blade_x + 2, spike_y, colors["metal_high"])


def draw_spear(canvas, style, colors, profile):
    w, h = canvas.width, canvas.height
    mid = h // 2
    shaft = w - 3
    shaft_thick = max(1, profile["shaft"])
    blade_thick = max(1, profile["blade"])
    blade_half = blade_thick // 2
    canvas.rect(0, mid - shaft_thick // 2, shaft, shaft_thick, colors["wood"])
    for dy in range(-blade_half, blade_thick - blade_half):
        canvas.set_px(w - 2, mid + dy, colors["metal_light"])
    canvas.set_px(w - 1, mid, colors["metal_high"])


def draw_shield(canvas, style, colors, profile):
    w, h = canvas.width, canvas.height
    canvas.rect(2, 2, w - 4, h - 4, colors["wood"])
    for x in range(1, w - 1):
        canvas.set_px(x, 1, colors["metal_dark"])
        canvas.set_px(x, h - 2, colors["metal_dark"])
    for y in range(1, h - 1):
        canvas.set_px(1, y, colors["metal_dark"])
        canvas.set_px(w - 2, y, colors["metal_dark"])
    canvas.rect(w // 2 - 1, h // 2 - 1, 2, 2, colors["metal_high"])
    if profile["round_shield"]:
        canvas.set_px(2, 2, (0, 0, 0, 0))
        canvas.set_px(w - 3, 2, (0, 0, 0, 0))
        canvas.set_px(2, h - 3, (0, 0, 0, 0))
        canvas.set_px(w - 3, h - 3, (0, 0, 0, 0))


def draw_unarmed(canvas, style, colors, profile):
    # Intentionally blank: unarmed sprites should not render a weapon icon.
    return


def weapon_variant_params(mode, width):
    if mode == "highres":
        return {"length": max(6, width - width // 6), "thickness": max(2, width // 14), "head": max(4, width // 10)}
    if mode == "ink":
        return {"length": max(6, width - width // 6), "thickness": max(2, width // 16), "head": max(4, width // 11)}
    if mode == "profile":
        return {"length": max(6, width - 3), "thickness": 1, "head": 2}
    if mode == "chibi":
        return {"length": max(6, width - 6), "thickness": 3, "head": 4}
    if mode == "blocky":
        return {"length": max(6, width - 4), "thickness": 4, "head": 5}
    if mode == "silhouette":
        return {"length": max(6, width - 3), "thickness": 3, "head": 4}
    if mode == "lineart":
        return {"length": max(6, width - 3), "thickness": 1, "head": 2}
    return {"length": max(6, width - 4), "thickness": 2, "head": 3}


def weapon_variant_colors(mode, colors):
    if mode == "highres":
        return colors["wood"], colors["metal_light"], colors["metal_high"], colors["string"]
    if mode == "ink":
        return colors["wood"], colors["metal_light"], colors["metal_high"], colors["string"]
    if mode == "silhouette":
        return colors["metal_dark"], colors["metal_dark"], colors["metal_dark"], colors["metal_dark"]
    if mode == "lineart":
        return colors["metal_high"], colors["metal_high"], colors["metal_high"], colors["metal_high"]
    return colors["wood"], colors["metal_light"], colors["metal_high"], colors["string"]


def draw_weapon_variant(canvas, key, mode, colors):
    if key == "unarmed":
        return
    w, h = canvas.width, canvas.height
    mid = h // 2
    params = weapon_variant_params(mode, w)
    length = params["length"]
    thickness = params["thickness"]
    head = params["head"]
    shaft_color, metal_color, accent_color, string_color = weapon_variant_colors(mode, colors)
    outline_only = mode == "lineart"
    ink_mode = mode == "ink"

    def draw_handle(x0, y0, size, color):
        if outline_only:
            draw_rect_outline(canvas, x0, y0, size, size, color)
        else:
            canvas.rect(x0, y0, size, size, color)

    if key in ("basic", "small_swords", "large_swords"):
        long_bonus = 2 if key == "large_swords" else 0
        handle_len = max(2, length // 4)
        blade_len = length - handle_len + long_bonus
        if outline_only:
            canvas.line(0, mid, blade_len, mid, metal_color)
            canvas.line(handle_len, mid - 1, handle_len, mid + 1, metal_color)
            canvas.set_px(blade_len + 1, mid, accent_color)
        else:
            guard_w = max(2, thickness + thickness // 2)
            guard_h = max(2, thickness // 2)
            canvas.rect(0, mid - thickness // 2, handle_len, thickness, shaft_color)
            if mode in ("highres", "ink"):
                for x in range(0, handle_len, max(2, thickness // 2)):
                    canvas.set_px(x, mid - thickness // 2, accent_color)
                canvas.rect(handle_len - guard_w // 2, mid - guard_h // 2, guard_w, guard_h, metal_color)
                canvas.rect(0, mid - thickness // 2, max(2, thickness // 2), thickness, accent_color)
            canvas.rect(handle_len, mid - thickness // 2, blade_len, thickness, metal_color)
            if mode in ("highres", "ink"):
                canvas.rect(handle_len, mid - thickness // 2, blade_len, 1, accent_color)
                canvas.rect(handle_len, mid + thickness // 2 - 1, blade_len, 1, colors["metal_dark"])
            canvas.set_px(handle_len + blade_len, mid, accent_color)
    elif key == "double":
        handle_len = max(4, length - head * 2)
        if outline_only:
            canvas.line(0, mid, length, mid, metal_color)
            canvas.set_px(0, mid - 1, accent_color)
            canvas.set_px(length, mid + 1, accent_color)
        else:
            canvas.rect(head, mid - thickness // 2, handle_len, thickness, shaft_color)
            draw_handle(0, mid - head // 2, head, metal_color)
            draw_handle(head + handle_len, mid - head // 2, head, metal_color)
            if mode in ("highres", "ink"):
                for x in range(head, head + handle_len, max(2, thickness // 2)):
                    canvas.set_px(x, mid - thickness // 2, accent_color)
    elif key == "axes":
        handle_len = max(4, length - head)
        if outline_only:
            canvas.line(0, mid, handle_len, mid, shaft_color)
            draw_rect_outline(canvas, handle_len, mid - head // 2, head, head, metal_color)
        else:
            canvas.rect(0, mid - thickness // 2, handle_len, thickness, shaft_color)
            canvas.rect(handle_len, mid - head // 2, head, head, metal_color)
            if mode in ("highres", "ink"):
                canvas.rect(handle_len + 1, mid - head // 2 + 1, head - 2, 1, accent_color)
                canvas.set_px(handle_len + head - 1, mid - head // 2, accent_color)
                canvas.set_px(handle_len + head - 1, mid + head // 2, accent_color)
            canvas.set_px(handle_len + head - 1, mid, accent_color)
    elif key == "blunt":
        handle_len = max(4, length - head)
        if outline_only:
            canvas.line(0, mid, handle_len, mid, shaft_color)
            draw_circle_outline(canvas, handle_len + head // 2, mid, head // 2, metal_color)
        else:
            canvas.rect(0, mid - thickness // 2, handle_len, thickness, shaft_color)
            draw_circle(canvas, handle_len + head // 2, mid, head // 2, metal_color)
            if mode in ("highres", "ink"):
                canvas.set_px(handle_len + head // 2, mid - head // 2 + 1, accent_color)
                canvas.set_px(handle_len + head // 2 - 1, mid, colors["metal_dark"])
            canvas.set_px(handle_len + head // 2, mid - 1, accent_color)
    elif key in ("polearms", "spears"):
        handle_len = max(6, length)
        if outline_only:
            canvas.line(0, mid, handle_len, mid, shaft_color)
            canvas.line(handle_len - 1, mid, handle_len + 2, mid - 1, metal_color)
            canvas.line(handle_len - 1, mid, handle_len + 2, mid + 1, metal_color)
        else:
            canvas.rect(0, mid - thickness // 2, handle_len, thickness, shaft_color)
            canvas.set_px(handle_len + 1, mid, metal_color)
            canvas.set_px(handle_len, mid - 1, metal_color)
            canvas.set_px(handle_len, mid + 1, metal_color)
            if key == "polearms":
                canvas.set_px(handle_len - 2, mid - 2, metal_color)
                canvas.set_px(handle_len - 2, mid + 2, metal_color)
            if mode in ("highres", "ink"):
                canvas.rect(handle_len - 3, mid - thickness // 2, 2, thickness, colors["metal_dark"])
                for x in range(int(2 * thickness), handle_len, int(3 * thickness)):
                    canvas.rect(x, mid - thickness // 2, 1, thickness, accent_color)
    elif key == "bows":
        if outline_only:
            canvas.line(1, 1, w - 2, mid, metal_color)
            canvas.line(1, h - 2, w - 2, mid, metal_color)
            canvas.line(1, 1, 1, h - 2, string_color)
        else:
            canvas.line(1, 1, w - 2, mid, metal_color)
            canvas.line(1, h - 2, w - 2, mid, metal_color)
            canvas.line(1, 1, 1, h - 2, string_color)
            if mode in ("highres", "ink"):
                canvas.line(2, 1, 2, h - 2, accent_color)
                canvas.rect(w // 2 - 1, mid - thickness // 2, 3, thickness, shaft_color)
            canvas.set_px(w - 2, mid, accent_color)
    elif key == "crossbows":
        if outline_only:
            canvas.line(1, mid, w - 2, mid, metal_color)
            canvas.line(w // 2, mid - 2, w // 2, mid + 2, metal_color)
        else:
            canvas.rect(1, mid - thickness // 2, w - 3, thickness, metal_color)
            canvas.rect(w // 2 - 1, mid - 2, 3, 4, shaft_color)
            if mode in ("highres", "ink"):
                canvas.rect(w // 2 - 2, mid - 1, 5, 1, accent_color)
                canvas.rect(w // 2 - 1, mid + 2, 3, 2, colors["metal_dark"])
            canvas.set_px(w - 2, mid - 1, accent_color)
    elif key == "ensnaring":
        for x in range(1, w - 1, 2):
            canvas.line(x, 1, x, h - 2, string_color)
        for y in range(1, h - 1, 2):
            canvas.line(1, y, w - 2, y, string_color)
        if not outline_only:
            canvas.rect(0, mid - 1, 2, 2, shaft_color)
    elif key == "lashes":
        for x in range(1, w - 1):
            y = mid + ((x // 2) % 2) - 1
            canvas.set_px(x, y, string_color)
        if not outline_only:
            canvas.rect(0, mid - 1, 2, 2, shaft_color)
    elif key == "shields":
        if outline_only:
            draw_rect_outline(canvas, 2, 2, w - 4, h - 4, metal_color)
        else:
            canvas.rect(2, 2, w - 4, h - 4, metal_color)
            canvas.set_px(w // 2, h // 2, accent_color)
            if mode in ("highres", "ink"):
                canvas.rect(2, 2, w - 4, 1, colors["metal_dark"])
                canvas.rect(2, h - 3, w - 4, 1, colors["metal_dark"])
                canvas.rect(2, 2, 1, h - 4, colors["metal_dark"])
                canvas.rect(w - 3, 2, 1, h - 4, colors["metal_dark"])
    else:
        if outline_only:
            canvas.line(0, mid, w - 1, mid, metal_color)
        else:
            canvas.rect(0, mid - thickness // 2, length, thickness, metal_color)


def render_weapon_variant(spec, style, colors):
    if style.weapon_mode in ("highres", "ink"):
        canvas = Canvas(spec.width * HIGHRES_SCALE, spec.height * HIGHRES_SCALE)
    else:
        canvas = Canvas(spec.width, spec.height)
    draw_weapon_variant(canvas, spec.key, style.weapon_mode, colors)
    if style.weapon_mode == "ink":
        apply_hatching(canvas, max(5, HIGHRES_SCALE), style.outline_color, 0.5)
        apply_outline_thick(canvas, style.outline_color, max(2, HIGHRES_SCALE // 6))
    elif style.outline and style.weapon_mode not in ("lineart", "silhouette"):
        apply_outline(canvas, style.outline_color)
    return canvas.width, canvas.height, canvas.pixels


def render_weapon(spec, style, colors):
    if getattr(style, "weapon_mode", "classic") != "classic":
        return render_weapon_variant(spec, style, colors)
    canvas = Canvas(spec.width, spec.height)
    profile = weapon_profile_for_style(style.key)
    spec.draw_fn(canvas, style, colors, profile)
    if style.outline:
        apply_outline(canvas, style.outline_color)
    return canvas.width, canvas.height, canvas.pixels


def compose_sheet(items, columns, padding=2, bg=(12, 12, 16, 255)):
    if not items:
        return 1, 1, bytearray([0, 0, 0, 0])
    item_w = items[0][0]
    item_h = items[0][1]
    rows = int(math.ceil(len(items) / columns))
    width = columns * item_w + padding * (columns + 1)
    height = rows * item_h + padding * (rows + 1)
    canvas = Canvas(width, height, bg=bg)
    for idx, (w, h, pixels) in enumerate(items):
        row = idx // columns
        col = idx % columns
        x = padding + col * (item_w + padding)
        y = padding + row * (item_h + padding)
        canvas.blit(x, y, w, h, pixels)
    return canvas.width, canvas.height, canvas.pixels


def build_race_specs():
    colors = {
        "skin_fair": (230, 208, 184),
        "skin_ruddy": (218, 184, 160),
        "skin_olive": (196, 156, 116),
        "skin_tan": (198, 150, 108),
        "skin_brown": (174, 126, 94),
        "skin_frost": (216, 212, 216),
        "skin_green": (130, 150, 110),
        "skin_grayblue": (170, 178, 190),
        "hair_black": (34, 30, 34),
        "hair_brown": (94, 60, 40),
        "hair_blond": (198, 170, 88),
        "hair_red": (150, 72, 44),
        "hair_dark": (56, 48, 54),
        "eyes_blue": (86, 136, 190),
        "eyes_green": (96, 160, 108),
        "eyes_hazel": (156, 122, 64),
        "eyes_brown": (120, 88, 60),
        "eyes_dark": (60, 60, 64),
        "eyes_orange": (214, 140, 40),
    }

    return [
        SpriteSpec(
            race_id="armeroci",
            skin=colors["skin_ruddy"],
            hair=colors["hair_red"],
            eyes=colors["eyes_green"],
            tunic=(70, 110, 150),
            pants=(70, 70, 90),
            boots=(42, 34, 28),
            accent=(186, 150, 96),
            height=1.08,
            build=0.95,
            ear="pointed",
            nose="small",
            brow="soft",
            hair_style="long",
            freckles=True,
            rugged=False,
            tattoo=None,
            face_style="human",
        ),
        SpriteSpec(
            race_id="fymblwngen",
            skin=colors["skin_fair"],
            hair=colors["hair_blond"],
            eyes=colors["eyes_blue"],
            tunic=(84, 96, 150),
            pants=(86, 78, 74),
            boots=(46, 38, 30),
            accent=(176, 144, 96),
            height=1.1,
            build=1.2,
            ear="round",
            nose="prominent",
            brow="heavy",
            hair_style="cropped",
            freckles=False,
            rugged=False,
            tattoo=None,
            face_style="human",
        ),
        SpriteSpec(
            race_id="ithican",
            skin=colors["skin_olive"],
            hair=colors["hair_dark"],
            eyes=colors["eyes_dark"],
            tunic=(118, 84, 84),
            pants=(90, 78, 74),
            boots=(48, 40, 34),
            accent=(170, 140, 100),
            height=1.0,
            build=1.0,
            ear="round",
            nose="small",
            brow="soft",
            hair_style="short",
            freckles=False,
            rugged=False,
            tattoo=None,
            face_style="human",
        ),
        SpriteSpec(
            race_id="kanian",
            skin=colors["skin_fair"],
            hair=colors["hair_brown"],
            eyes=colors["eyes_blue"],
            tunic=(74, 104, 88),
            pants=(80, 72, 66),
            boots=(44, 36, 32),
            accent=(176, 144, 92),
            height=1.05,
            build=1.12,
            ear="round",
            nose="prominent",
            brow="heavy",
            hair_style="short",
            freckles=False,
            rugged=True,
            tattoo=None,
            face_style="human",
        ),
        SpriteSpec(
            race_id="katlakehan",
            skin=colors["skin_brown"],
            hair=colors["hair_black"],
            eyes=colors["eyes_hazel"],
            tunic=(130, 96, 64),
            pants=(88, 76, 62),
            boots=(42, 34, 28),
            accent=(170, 130, 84),
            height=1.0,
            build=1.0,
            ear="round",
            nose="prominent",
            brow="soft",
            hair_style="long",
            freckles=False,
            rugged=False,
            tattoo=None,
            face_style="human",
        ),
        SpriteSpec(
            race_id="midlander",
            skin=colors["skin_tan"],
            hair=colors["hair_brown"],
            eyes=colors["eyes_brown"],
            tunic=(60, 72, 120),
            pants=(82, 74, 70),
            boots=(44, 36, 32),
            accent=(170, 136, 90),
            height=1.02,
            build=1.02,
            ear="round",
            nose="small",
            brow="soft",
            hair_style="short",
            freckles=False,
            rugged=True,
            tattoo=None,
            face_style="human",
        ),
        SpriteSpec(
            race_id="pather",
            skin=colors["skin_ruddy"],
            hair=colors["hair_brown"],
            eyes=colors["eyes_hazel"],
            tunic=(96, 96, 110),
            pants=(86, 82, 78),
            boots=(44, 36, 30),
            accent=(170, 140, 90),
            height=1.0,
            build=1.0,
            ear="round",
            nose="small",
            brow="soft",
            hair_style="short",
            freckles=False,
            rugged=False,
            tattoo=None,
            face_style="human",
        ),
        SpriteSpec(
            race_id="vetlander",
            skin=colors["skin_fair"],
            hair=colors["hair_black"],
            eyes=colors["eyes_hazel"],
            tunic=(88, 82, 74),
            pants=(78, 70, 64),
            boots=(40, 34, 28),
            accent=(160, 130, 80),
            height=0.95,
            build=1.1,
            ear="round",
            nose="prominent",
            brow="heavy",
            hair_style="cropped",
            freckles=False,
            rugged=True,
            tattoo=None,
            face_style="human",
        ),
        SpriteSpec(
            race_id="limmtrig",
            skin=colors["skin_green"],
            hair=colors["hair_black"],
            eyes=colors["eyes_orange"],
            tunic=(86, 92, 66),
            pants=(74, 70, 62),
            boots=(40, 34, 28),
            accent=(150, 120, 80),
            height=1.12,
            build=1.25,
            ear="long",
            nose="bulbous",
            brow="heavy",
            hair_style="wild",
            freckles=False,
            rugged=True,
            tattoo=None,
            face_style="classic",
        ),
        SpriteSpec(
            race_id="cirodes",
            skin=colors["skin_grayblue"],
            hair=colors["hair_black"],
            eyes=colors["eyes_orange"],
            tunic=(88, 78, 64),
            pants=(72, 68, 62),
            boots=(38, 32, 28),
            accent=(170, 120, 82),
            height=1.14,
            build=1.28,
            ear="long",
            nose="small",
            brow="heavy",
            hair_style="wild",
            freckles=False,
            rugged=True,
            tattoo=None,
            face_style="classic",
        ),
        SpriteSpec(
            race_id="qaraz",
            skin=colors["skin_olive"],
            hair=colors["hair_dark"],
            eyes=colors["eyes_green"],
            tunic=(74, 92, 112),
            pants=(82, 74, 70),
            boots=(40, 34, 30),
            accent=(188, 152, 96),
            height=1.0,
            build=0.98,
            ear="pointed",
            nose="prominent",
            brow="soft",
            hair_style="long",
            freckles=False,
            rugged=False,
            tattoo=(70, 104, 136),
            face_style="classic",
        ),
        SpriteSpec(
            race_id="vorova_female",
            skin=colors["skin_grayblue"],
            hair=colors["hair_dark"],
            eyes=colors["eyes_blue"],
            tunic=(70, 96, 132),
            pants=(80, 80, 86),
            boots=(38, 34, 30),
            accent=(120, 140, 180),
            height=1.06,
            build=1.05,
            ear="pointed",
            nose="small",
            brow="soft",
            hair_style="long",
            freckles=False,
            rugged=False,
            tattoo=(60, 90, 150),
            face_style="classic",
        ),
        SpriteSpec(
            race_id="vorova_male",
            skin=colors["skin_grayblue"],
            hair=colors["hair_black"],
            eyes=colors["eyes_blue"],
            tunic=(90, 80, 84),
            pants=(72, 72, 76),
            boots=(38, 32, 28),
            accent=(160, 90, 90),
            height=1.12,
            build=1.3,
            ear="round",
            nose="bulbous",
            brow="heavy",
            hair_style="cropped",
            freckles=False,
            rugged=True,
            tattoo=(160, 70, 70),
            face_style="classic",
        ),
    ]


def build_hobgoblin_spec():
    return SpriteSpec(
        race_id="hobgoblin",
        skin=(120, 150, 90),
        hair=(34, 30, 34),
        eyes=(180, 90, 60),
        tunic=(130, 70, 60),
        pants=(86, 74, 66),
        boots=(40, 32, 28),
        accent=(150, 120, 80),
        height=1.04,
        build=1.1,
        ear="pointed",
        nose="prominent",
        brow="heavy",
        hair_style="short",
        freckles=False,
        rugged=True,
        tattoo=None,
        face_style="classic",
    )


def build_weapon_specs():
    return [
        WeaponSpec("unarmed", 10, 8, draw_unarmed),
        WeaponSpec("axes", 16, 7, draw_axe),
        WeaponSpec("basic", 16, 6, draw_sword),
        WeaponSpec("blunt", 15, 7, draw_blunt),
        WeaponSpec("bows", 10, 18, draw_bow),
        WeaponSpec("crossbows", 18, 10, draw_crossbow),
        WeaponSpec("double", 18, 6, draw_double),
        WeaponSpec("ensnaring", 16, 10, draw_net),
        WeaponSpec("lashes", 18, 6, draw_whip),
        WeaponSpec(
            "large_swords",
            20,
            6,
            lambda c, s, col, prof: draw_sword(c, s, col, prof, True),
        ),
        WeaponSpec("small_swords", 14, 6, draw_sword),
        WeaponSpec("polearms", 22, 6, draw_polearm),
        WeaponSpec("spears", 22, 6, draw_spear),
        WeaponSpec("shields", 12, 12, draw_shield),
    ]


def weapon_colors():
    return {
        "metal_light": rgba((200, 200, 210)),
        "metal_dark": rgba((120, 120, 130)),
        "metal_high": rgba((232, 232, 238)),
        "wood": rgba((118, 80, 50)),
        "leather": rgba((74, 54, 36)),
        "string": rgba((200, 186, 150)),
    }


def weapon_profile_for_style(style_key):
    profile = {
        "blade": 2,
        "shaft": 2,
        "guard": 2,
        "head": 3,
        "curve": 1.0,
        "round_shield": False,
        "spike": False,
    }
    if style_key == "style02_flat":
        profile.update({"blade": 3, "shaft": 3, "guard": 3, "head": 4})
    elif style_key == "style03_noir":
        profile.update({"blade": 1, "shaft": 1, "guard": 1, "head": 2})
    elif style_key == "style04_pastel":
        profile.update({"blade": 2, "shaft": 2, "guard": 2, "head": 2, "round_shield": True})
    elif style_key == "style05_warm":
        profile.update({"blade": 2, "shaft": 2, "guard": 3, "head": 3})
    elif style_key == "style06_cool":
        profile.update({"blade": 2, "shaft": 2, "guard": 2, "head": 3, "curve": 1.1})
    elif style_key == "style07_neon":
        profile.update({"blade": 2, "shaft": 2, "guard": 2, "head": 3, "spike": True})
    elif style_key == "style08_comic":
        profile.update({"blade": 3, "shaft": 3, "guard": 4, "head": 4})
    elif style_key == "style09_dither":
        profile.update({"blade": 2, "shaft": 2, "guard": 2, "head": 3, "spike": True})
    elif style_key == "style11_illustrated":
        profile.update({"blade": 2, "shaft": 2, "guard": 2, "head": 3, "curve": 1.05})
    return profile


def build_style_presets():
    return [
        Style(
            key="style01_classic",
            detail=5,
            outline=True,
            shading=True,
            outline_color=DEFAULT_OUTLINE,
            light_factor=1.18,
            dark_factor=0.75,
            darker_factor=0.55,
            highlight_factor=1.32,
            tint=None,
            tint_strength=0.0,
            saturation=1.0,
            brightness=1.0,
            contrast=1.0,
            posterize_levels=0,
            dither=False,
            dither_strength=0,
            noise=False,
            noise_strength=0,
            blur_radius=0,
            smooth_scale=False,
            sprite_scale=2,
            weapon_scale=2,
            edge_soften=False,
            edge_soften_strength=0.0,
            gradient_top=1.0,
            gradient_bottom=1.0,
        ),
        Style(
            key="style02_flat",
            detail=2,
            outline=True,
            shading=False,
            outline_color=rgba((30, 26, 28)),
            light_factor=1.0,
            dark_factor=1.0,
            darker_factor=1.0,
            highlight_factor=1.0,
            tint=None,
            tint_strength=0.0,
            saturation=1.05,
            brightness=1.02,
            contrast=1.0,
            posterize_levels=4,
            dither=False,
            dither_strength=0,
            noise=False,
            noise_strength=0,
            blur_radius=0,
            smooth_scale=False,
            sprite_scale=2,
            weapon_scale=2,
            edge_soften=False,
            edge_soften_strength=0.0,
            gradient_top=1.0,
            gradient_bottom=1.0,
        ),
        Style(
            key="style03_noir",
            detail=4,
            outline=True,
            shading=True,
            outline_color=rgba((8, 8, 8)),
            light_factor=1.1,
            dark_factor=0.65,
            darker_factor=0.5,
            highlight_factor=1.2,
            tint=None,
            tint_strength=0.0,
            saturation=0.0,
            brightness=0.95,
            contrast=1.4,
            posterize_levels=3,
            dither=False,
            dither_strength=0,
            noise=False,
            noise_strength=0,
            blur_radius=0,
            smooth_scale=False,
            sprite_scale=2,
            weapon_scale=2,
            edge_soften=False,
            edge_soften_strength=0.0,
            gradient_top=1.0,
            gradient_bottom=1.0,
        ),
        Style(
            key="style04_pastel",
            detail=3,
            outline=True,
            shading=True,
            outline_color=rgba((90, 90, 100)),
            light_factor=1.08,
            dark_factor=0.9,
            darker_factor=0.82,
            highlight_factor=1.12,
            tint=None,
            tint_strength=0.0,
            saturation=0.6,
            brightness=1.18,
            contrast=0.85,
            posterize_levels=0,
            dither=False,
            dither_strength=0,
            noise=False,
            noise_strength=0,
            blur_radius=0,
            smooth_scale=True,
            sprite_scale=2,
            weapon_scale=2,
            edge_soften=False,
            edge_soften_strength=0.0,
            gradient_top=1.0,
            gradient_bottom=1.0,
        ),
        Style(
            key="style05_warm",
            detail=4,
            outline=True,
            shading=True,
            outline_color=rgba((52, 36, 28)),
            light_factor=1.16,
            dark_factor=0.72,
            darker_factor=0.55,
            highlight_factor=1.28,
            tint=(220, 150, 100),
            tint_strength=0.22,
            saturation=1.1,
            brightness=1.05,
            contrast=1.0,
            posterize_levels=0,
            dither=False,
            dither_strength=0,
            noise=False,
            noise_strength=0,
            blur_radius=0,
            smooth_scale=False,
            sprite_scale=2,
            weapon_scale=2,
            edge_soften=False,
            edge_soften_strength=0.0,
            gradient_top=1.0,
            gradient_bottom=1.0,
        ),
        Style(
            key="style06_cool",
            detail=4,
            outline=True,
            shading=True,
            outline_color=rgba((30, 42, 60)),
            light_factor=1.14,
            dark_factor=0.7,
            darker_factor=0.55,
            highlight_factor=1.26,
            tint=(90, 140, 200),
            tint_strength=0.2,
            saturation=0.9,
            brightness=1.0,
            contrast=1.05,
            posterize_levels=0,
            dither=False,
            dither_strength=0,
            noise=False,
            noise_strength=0,
            blur_radius=0,
            smooth_scale=False,
            sprite_scale=2,
            weapon_scale=2,
            edge_soften=False,
            edge_soften_strength=0.0,
            gradient_top=1.0,
            gradient_bottom=1.0,
        ),
        Style(
            key="style07_neon",
            detail=4,
            outline=True,
            shading=True,
            outline_color=rgba((10, 210, 230)),
            light_factor=1.25,
            dark_factor=0.72,
            darker_factor=0.55,
            highlight_factor=1.4,
            tint=(60, 220, 240),
            tint_strength=0.3,
            saturation=1.6,
            brightness=1.1,
            contrast=1.1,
            posterize_levels=6,
            dither=True,
            dither_strength=6,
            noise=False,
            noise_strength=0,
            blur_radius=0,
            smooth_scale=True,
            sprite_scale=2,
            weapon_scale=2,
            edge_soften=False,
            edge_soften_strength=0.0,
            gradient_top=1.0,
            gradient_bottom=1.0,
        ),
        Style(
            key="style08_comic",
            detail=4,
            outline=True,
            shading=True,
            outline_color=rgba((12, 12, 12)),
            light_factor=1.2,
            dark_factor=0.7,
            darker_factor=0.52,
            highlight_factor=1.3,
            tint=None,
            tint_strength=0.0,
            saturation=1.0,
            brightness=1.0,
            contrast=1.2,
            posterize_levels=5,
            dither=False,
            dither_strength=0,
            noise=False,
            noise_strength=0,
            blur_radius=0,
            smooth_scale=False,
            sprite_scale=2,
            weapon_scale=2,
            edge_soften=False,
            edge_soften_strength=0.0,
            gradient_top=1.0,
            gradient_bottom=1.0,
        ),
        Style(
            key="style09_dither",
            detail=3,
            outline=True,
            shading=True,
            outline_color=rgba((28, 24, 28)),
            light_factor=1.12,
            dark_factor=0.72,
            darker_factor=0.55,
            highlight_factor=1.22,
            tint=None,
            tint_strength=0.0,
            saturation=1.0,
            brightness=1.0,
            contrast=1.0,
            posterize_levels=0,
            dither=True,
            dither_strength=10,
            noise=True,
            noise_strength=6,
            blur_radius=0,
            smooth_scale=False,
            sprite_scale=2,
            weapon_scale=2,
            edge_soften=False,
            edge_soften_strength=0.0,
            gradient_top=1.0,
            gradient_bottom=1.0,
        ),
        Style(
            key="style10_painted",
            detail=5,
            outline=True,
            shading=True,
            outline_color=rgba((40, 40, 46)),
            light_factor=1.14,
            dark_factor=0.8,
            darker_factor=0.65,
            highlight_factor=1.18,
            tint=(200, 180, 160),
            tint_strength=0.08,
            saturation=0.9,
            brightness=1.05,
            contrast=0.9,
            posterize_levels=0,
            dither=False,
            dither_strength=0,
            noise=True,
            noise_strength=8,
            blur_radius=1,
            smooth_scale=True,
            sprite_scale=2,
            weapon_scale=2,
            edge_soften=False,
            edge_soften_strength=0.0,
            gradient_top=1.0,
            gradient_bottom=1.0,
        ),
        Style(
            key="style11_illustrated",
            detail=5,
            outline=False,
            shading=True,
            outline_color=rgba((60, 60, 70)),
            light_factor=1.12,
            dark_factor=0.9,
            darker_factor=0.78,
            highlight_factor=1.16,
            tint=(190, 170, 150),
            tint_strength=0.12,
            saturation=0.85,
            brightness=1.1,
            contrast=0.78,
            posterize_levels=0,
            dither=False,
            dither_strength=0,
            noise=True,
            noise_strength=14,
            blur_radius=3,
            smooth_scale=True,
            sprite_scale=6,
            weapon_scale=6,
            edge_soften=True,
            edge_soften_strength=0.45,
            gradient_top=1.06,
            gradient_bottom=0.92,
        ),
        Style(
            key="style12_profile",
            detail=2,
            outline=False,
            shading=False,
            outline_color=rgba((20, 20, 28)),
            light_factor=1.0,
            dark_factor=1.0,
            darker_factor=1.0,
            highlight_factor=1.0,
            tint=None,
            tint_strength=0.0,
            saturation=1.0,
            brightness=1.0,
            contrast=1.0,
            posterize_levels=0,
            dither=False,
            dither_strength=0,
            noise=False,
            noise_strength=0,
            blur_radius=0,
            smooth_scale=False,
            sprite_scale=2,
            weapon_scale=2,
            edge_soften=False,
            edge_soften_strength=0.0,
            gradient_top=1.0,
            gradient_bottom=1.0,
            render_mode="profile",
            weapon_mode="profile",
        ),
        Style(
            key="style13_chibi",
            detail=3,
            outline=True,
            shading=True,
            outline_color=rgba((24, 22, 28)),
            light_factor=1.1,
            dark_factor=0.85,
            darker_factor=0.7,
            highlight_factor=1.15,
            tint=None,
            tint_strength=0.0,
            saturation=1.05,
            brightness=1.05,
            contrast=1.0,
            posterize_levels=0,
            dither=False,
            dither_strength=0,
            noise=False,
            noise_strength=0,
            blur_radius=0,
            smooth_scale=False,
            sprite_scale=3,
            weapon_scale=3,
            edge_soften=False,
            edge_soften_strength=0.0,
            gradient_top=1.0,
            gradient_bottom=1.0,
            render_mode="chibi",
            weapon_mode="chibi",
        ),
        Style(
            key="style14_blocky",
            detail=2,
            outline=True,
            shading=False,
            outline_color=rgba((18, 18, 20)),
            light_factor=1.0,
            dark_factor=1.0,
            darker_factor=1.0,
            highlight_factor=1.0,
            tint=None,
            tint_strength=0.0,
            saturation=1.0,
            brightness=1.0,
            contrast=1.0,
            posterize_levels=3,
            dither=False,
            dither_strength=0,
            noise=False,
            noise_strength=0,
            blur_radius=0,
            smooth_scale=False,
            sprite_scale=2,
            weapon_scale=2,
            edge_soften=False,
            edge_soften_strength=0.0,
            gradient_top=1.0,
            gradient_bottom=1.0,
            render_mode="blocky",
            weapon_mode="blocky",
        ),
        Style(
            key="style15_silhouette",
            detail=1,
            outline=False,
            shading=False,
            outline_color=rgba((0, 0, 0)),
            light_factor=1.0,
            dark_factor=1.0,
            darker_factor=1.0,
            highlight_factor=1.0,
            tint=None,
            tint_strength=0.0,
            saturation=0.9,
            brightness=0.9,
            contrast=1.2,
            posterize_levels=0,
            dither=False,
            dither_strength=0,
            noise=False,
            noise_strength=0,
            blur_radius=0,
            smooth_scale=False,
            sprite_scale=2,
            weapon_scale=2,
            edge_soften=False,
            edge_soften_strength=0.0,
            gradient_top=1.0,
            gradient_bottom=1.0,
            render_mode="silhouette",
            weapon_mode="silhouette",
        ),
        Style(
            key="style16_lineart",
            detail=2,
            outline=False,
            shading=False,
            outline_color=rgba((230, 230, 230)),
            light_factor=1.0,
            dark_factor=1.0,
            darker_factor=1.0,
            highlight_factor=1.0,
            tint=None,
            tint_strength=0.0,
            saturation=0.8,
            brightness=1.0,
            contrast=1.0,
            posterize_levels=0,
            dither=False,
            dither_strength=0,
            noise=False,
            noise_strength=0,
            blur_radius=0,
            smooth_scale=False,
            sprite_scale=2,
            weapon_scale=2,
            edge_soften=False,
            edge_soften_strength=0.0,
            gradient_top=1.0,
            gradient_bottom=1.0,
            render_mode="lineart",
            weapon_mode="lineart",
        ),
        Style(
            key="style17_highres",
            detail=4,
            outline=True,
            shading=True,
            outline_color=rgba((18, 16, 14)),
            light_factor=1.1,
            dark_factor=0.7,
            darker_factor=0.5,
            highlight_factor=1.25,
            tint=None,
            tint_strength=0.0,
            saturation=0.35,
            brightness=1.08,
            contrast=1.2,
            posterize_levels=0,
            dither=False,
            dither_strength=0,
            noise=False,
            noise_strength=0,
            blur_radius=0,
            smooth_scale=False,
            sprite_scale=1,
            weapon_scale=1,
            edge_soften=False,
            edge_soften_strength=0.0,
            gradient_top=1.0,
            gradient_bottom=1.0,
            render_mode="ink",
            weapon_mode="ink",
        ),
    ]


def generate_style_assets(style, races, hobgoblin, weapon_specs, colors):
    race_images = []
    race_pained_images = []
    race_assets = {}
    race_pained_assets = {}
    weapon_palette = stylize_palette(colors, style)
    for spec in races:
        w, h, pixels = render_character(spec, style)
        if style.smooth_scale:
            w, h, pixels = scale_pixels_smooth(w, h, pixels, style.sprite_scale)
        else:
            w, h, pixels = scale_pixels(w, h, pixels, style.sprite_scale)
        w, h, pixels = apply_postprocess_pixels(w, h, pixels, style)
        race_assets[spec.race_id] = (w, h, pixels)
        race_images.append((w, h, pixels))
        pw, ph, ppixels = render_character_downed(spec, style)
        if style.smooth_scale:
            pw, ph, ppixels = scale_pixels_smooth(pw, ph, ppixels, style.sprite_scale)
        else:
            pw, ph, ppixels = scale_pixels(pw, ph, ppixels, style.sprite_scale)
        pw, ph, ppixels = apply_postprocess_pixels(pw, ph, ppixels, style)
        race_pained_assets[spec.race_id] = (pw, ph, ppixels)
        race_pained_images.append((pw, ph, ppixels))

    enemy_w, enemy_h, enemy_pixels = render_character(hobgoblin, style)
    if style.smooth_scale:
        enemy_w, enemy_h, enemy_pixels = scale_pixels_smooth(
            enemy_w, enemy_h, enemy_pixels, style.sprite_scale
        )
    else:
        enemy_w, enemy_h, enemy_pixels = scale_pixels(
            enemy_w, enemy_h, enemy_pixels, style.sprite_scale
        )
    enemy_w, enemy_h, enemy_pixels = apply_postprocess_pixels(
        enemy_w, enemy_h, enemy_pixels, style
    )
    enemy_pw, enemy_ph, enemy_ppixels = render_character_downed(hobgoblin, style)
    if style.smooth_scale:
        enemy_pw, enemy_ph, enemy_ppixels = scale_pixels_smooth(
            enemy_pw, enemy_ph, enemy_ppixels, style.sprite_scale
        )
    else:
        enemy_pw, enemy_ph, enemy_ppixels = scale_pixels(
            enemy_pw, enemy_ph, enemy_ppixels, style.sprite_scale
        )
    enemy_pw, enemy_ph, enemy_ppixels = apply_postprocess_pixels(
        enemy_pw, enemy_ph, enemy_ppixels, style
    )

    weapon_images = []
    weapon_assets = {}
    for weapon in weapon_specs:
        w, h, pixels = render_weapon(weapon, style, weapon_palette)
        if style.smooth_scale:
            w, h, pixels = scale_pixels_smooth(w, h, pixels, style.weapon_scale)
        else:
            w, h, pixels = scale_pixels(w, h, pixels, style.weapon_scale)
        w, h, pixels = apply_postprocess_pixels(w, h, pixels, style)
        weapon_assets[weapon.key] = (w, h, pixels)
        weapon_images.append((w, h, pixels))

    return (
        race_assets,
        race_pained_assets,
        weapon_assets,
        (enemy_w, enemy_h, enemy_pixels),
        (enemy_pw, enemy_ph, enemy_ppixels),
        race_images,
        race_pained_images,
        weapon_images,
    )


def generate():
    default_races_dir = os.path.join("assets", "sprites", "races")
    default_enemies_dir = os.path.join("assets", "sprites", "enemies")
    default_weapons_dir = os.path.join("assets", "sprites", "weapons")
    styles_dir = os.path.join("assets", "sprites", "styles")
    screenshots_dir = os.path.join("screenshots")
    os.makedirs(default_races_dir, exist_ok=True)
    os.makedirs(default_enemies_dir, exist_ok=True)
    os.makedirs(default_weapons_dir, exist_ok=True)
    os.makedirs(styles_dir, exist_ok=True)
    os.makedirs(screenshots_dir, exist_ok=True)

    races = build_race_specs()
    hobgoblin = build_hobgoblin_spec()
    weapon_specs = build_weapon_specs()
    colors = weapon_colors()

    for style in build_style_presets():
        style_root = os.path.join(styles_dir, style.key)
        style_races_dir = os.path.join(style_root, "races")
        style_enemies_dir = os.path.join(style_root, "enemies")
        style_weapons_dir = os.path.join(style_root, "weapons")
        os.makedirs(style_races_dir, exist_ok=True)
        os.makedirs(style_enemies_dir, exist_ok=True)
        os.makedirs(style_weapons_dir, exist_ok=True)

        (
            race_assets,
            race_pained_assets,
            weapon_assets,
            enemy_asset,
            enemy_pained_asset,
            race_images,
            race_pained_images,
            weapon_images,
        ) = generate_style_assets(style, races, hobgoblin, weapon_specs, colors)

        race_sheet = compose_sheet(race_images, columns=6, padding=3)
        pained_sheet = compose_sheet(race_pained_images, columns=6, padding=3)
        weapon_sheet = compose_sheet(weapon_images, columns=7, padding=3)
        write_png(
            os.path.join(screenshots_dir, f"{style.key}_races.png"),
            race_sheet[0],
            race_sheet[1],
            race_sheet[2],
        )
        write_png(
            os.path.join(screenshots_dir, f"{style.key}_pained.png"),
            pained_sheet[0],
            pained_sheet[1],
            pained_sheet[2],
        )
        write_png(
            os.path.join(screenshots_dir, f"{style.key}_weapons.png"),
            weapon_sheet[0],
            weapon_sheet[1],
            weapon_sheet[2],
        )

        for race_id, (w, h, pixels) in race_assets.items():
            write_png(os.path.join(style_races_dir, f"{race_id}.png"), w, h, pixels)
        for race_id, (w, h, pixels) in race_pained_assets.items():
            write_png(
                os.path.join(style_races_dir, f"{race_id}_pained.png"), w, h, pixels
            )
        write_png(
            os.path.join(style_enemies_dir, "hobgoblin.png"),
            enemy_asset[0],
            enemy_asset[1],
            enemy_asset[2],
        )
        write_png(
            os.path.join(style_enemies_dir, "hobgoblin_pained.png"),
            enemy_pained_asset[0],
            enemy_pained_asset[1],
            enemy_pained_asset[2],
        )
        for key, (w, h, pixels) in weapon_assets.items():
            write_png(os.path.join(style_weapons_dir, f"{key}.png"), w, h, pixels)

        if style.key == DEFAULT_STYLE_KEY:
            for race_id, (w, h, pixels) in race_assets.items():
                write_png(os.path.join(default_races_dir, f"{race_id}.png"), w, h, pixels)
            for race_id, (w, h, pixels) in race_pained_assets.items():
                write_png(
                    os.path.join(default_races_dir, f"{race_id}_pained.png"),
                    w,
                    h,
                    pixels,
                )
            write_png(
                os.path.join(default_enemies_dir, "hobgoblin.png"),
                enemy_asset[0],
                enemy_asset[1],
                enemy_asset[2],
            )
            write_png(
                os.path.join(default_enemies_dir, "hobgoblin_pained.png"),
                enemy_pained_asset[0],
                enemy_pained_asset[1],
                enemy_pained_asset[2],
            )
            for key, (w, h, pixels) in weapon_assets.items():
                write_png(os.path.join(default_weapons_dir, f"{key}.png"), w, h, pixels)


if __name__ == "__main__":
    generate()
