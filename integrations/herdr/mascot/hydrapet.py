#!/usr/bin/env python3
"""
Vendored 2026-09-24 from the human's AI Studio export (hydra_pet/hydra.py). `hydra-mascot --pet`
imports `render_compact_dragon` from here; the rest is the standalone pet, run it as
`python3 hydrapet.py --size 20 --mode ansi`. Needs numpy.

🐉 HydraPet - Animated 3-Headed Emerald Dragon Pet for Modern Terminals (Ghostty / Kitty Protocol)
Features:
  - Kitty Graphics Protocol with safe chunking (RFC compliant <= 4096 bytes per chunk)
  - TrueColor ANSI half-block (▀) fallback mode
  - Two interchangeable visual modes:
      * HD Masterpiece Mode (160x160 canvas, rich lighting, scales, horns & fire)
      * Compact 20px Mode (24x24 pixel art micro-pet for small panes/splits)
  - Fluid living art animation: sinusoidal neck sway, breathing chest, flicking tongue
  - Random independent sleep: single heads nod off, close eyes, blow bubbles 🫧 and float Zzz
  - Global idle sleep: all 3 heads fall asleep when user is inactive; wake on any key
  - Dynamic roaring fire breath 🔥 on demand
  - Dragon petting interaction (hearts ♥ and sparkles ✨)
"""

import sys
import os
import time
import math
import random
import select
import signal
import base64
import struct
import zlib

# Check for numpy
try:
    import numpy as np
    HAS_NUMPY = True
except ImportError:
    HAS_NUMPY = False

# ==============================================================================
# Terminal Control & Kitty Graphics Protocol
# ==============================================================================

class TerminalEnv:
    @staticmethod
    def supports_kitty():
        term = os.environ.get("TERM", "").lower()
        term_prog = os.environ.get("TERM_PROGRAM", "").lower()
        kitty_win = os.environ.get("KITTY_WINDOW_ID", "")
        ghostty = "ghostty" in term or "ghostty" in term_prog or os.environ.get("GHOSTTY_RESOURCES_DIR")
        return bool(ghostty or kitty_win or "kitty" in term or "wezterm" in term_prog)

    @staticmethod
    def hide_cursor():
        sys.stdout.write("\x1b[?25l")
        sys.stdout.flush()

    @staticmethod
    def show_cursor():
        sys.stdout.write("\x1b[?25h")
        sys.stdout.flush()

    @staticmethod
    def enter_alt_screen():
        sys.stdout.write("\x1b[?1049h\x1b[H\x1b[2J")
        sys.stdout.flush()

    @staticmethod
    def leave_alt_screen():
        sys.stdout.write("\x1b[2J\x1b[?1049l\x1b[?25h")
        sys.stdout.flush()

    @staticmethod
    def move_cursor(r, c):
        sys.stdout.write(f"\x1b[{r};{c}H")

def encode_png(w, h, rgba_bytes):
    """Encodes raw RGBA bytes to standard PNG format without third-party libraries."""
    def chunk(tag, data):
        return struct.pack('>I', len(data)) + tag + data + struct.pack('>I', zlib.crc32(tag + data) & 0xffffffff)
    
    raw = bytearray()
    stride = w * 4
    for y in range(h):
        raw.append(0)  # No filter
        raw.extend(rgba_bytes[y * stride : (y + 1) * stride])
        
    idat = zlib.compress(bytes(raw), 4)
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 6, 0, 0, 0)) + chunk(b'IDAT', idat) + chunk(b'IEND', b'')

def render_kitty_chunks(png_data, cols, rows, image_id=1):
    """
    Transmits PNG image data using Kitty Graphics Protocol with chunking.
    Payload chunks <= 4096 bytes, m=1 until last chunk m=0.
    """
    b64 = base64.b64encode(png_data).decode('ascii')
    chunk_size = 4096
    total_len = len(b64)
    out = []

    for i in range(0, total_len, chunk_size):
        chunk_str = b64[i : i + chunk_size]
        is_last = (i + chunk_size >= total_len)
        m = 0 if is_last else 1
        if i == 0:
            out.append(f"\x1b_Ga=T,f=100,t=d,i={image_id},m={m},c={cols},r={rows};{chunk_str}\x1b\\")
        else:
            out.append(f"\x1b_Gm={m};{chunk_str}\x1b\\")

    return "".join(out)

def render_ansi_halfblocks(img_rgba, cols, rows):
    """
    Renders RGBA image to TrueColor unicode half-blocks (▀).
    Upper pixel = Foreground color, Lower pixel = Background color.
    """
    H, W = img_rgba.shape[:2]
    target_h = rows * 2
    target_w = cols
    step_y = H / target_h
    step_x = W / target_w

    small = np.zeros((target_h, target_w, 4), dtype=np.uint8)
    for y in range(target_h):
        for x in range(target_w):
            sy0, sy1 = int(y * step_y), int((y + 1) * step_y)
            sx0, sx1 = int(x * step_x), int((x + 1) * step_x)
            box = img_rgba[sy0:sy1, sx0:sx1]
            if box.size > 0:
                small[y, x] = np.mean(box.reshape(-1, 4), axis=0).astype(np.uint8)

    lines = []
    for y in range(0, target_h, 2):
        row_str = []
        for x in range(target_w):
            top = small[y, x]
            bot = small[y + 1, x] if y + 1 < target_h else [0, 0, 0, 0]
            ta, ba = top[3] / 255.0, bot[3] / 255.0
            tr, tg, tb = int(top[0] * ta), int(top[1] * ta), int(top[2] * ta)
            br, bg, bb = int(bot[0] * ba), int(bot[1] * ba), int(bot[2] * ba)
            row_str.append(f"\x1b[38;2;{tr};{tg};{tb}m\x1b[48;2;{br};{bg};{bb}m▀\x1b[0m")
        lines.append("".join(row_str))
    return "\n".join(lines)


# ==============================================================================
# Vectorized Fast Drawing Primitives (Bbox optimized, < 10ms per frame)
# ==============================================================================

def draw_circle(canvas, cx, cy, radius, col, alpha=1.0):
    H, W = canvas.shape[:2]
    r_ceil = int(math.ceil(radius + 1.2))
    x0 = max(0, int(cx - r_ceil))
    x1 = min(W, int(cx + r_ceil + 1))
    y0 = max(0, int(cy - r_ceil))
    y1 = min(H, int(cy + r_ceil + 1))
    if x0 >= x1 or y0 >= y1:
        return

    yy, xx = np.ogrid[y0:y1, x0:x1]
    dist_sq = (xx - cx)**2 + (yy - cy)**2
    r_sq = radius**2
    r_out_sq = (radius + 1.2)**2

    mask = np.clip((r_out_sq - dist_sq) / (r_out_sq - r_sq + 1e-5), 0.0, 1.0)
    a = mask * alpha
    if not np.any(a > 0.01):
        return

    sub = canvas[y0:y1, x0:x1]
    prev_a = sub[:, :, 3]
    out_a = a + prev_a * (1.0 - a)
    safe_a = np.where(out_a > 0.001, out_a, 1.0)

    for c in range(3):
        sub[:, :, c] = (col[c] * a + sub[:, :, c] * prev_a * (1.0 - a)) / safe_a
    sub[:, :, 3] = out_a

def draw_capsule(canvas, x1, y1, x2, y2, radius, col, alpha=1.0):
    H, W = canvas.shape[:2]
    r_ceil = int(math.ceil(radius + 1.2))
    x0 = max(0, int(min(x1, x2) - r_ceil))
    x1_b = min(W, int(max(x1, x2) + r_ceil + 1))
    y0 = max(0, int(min(y1, y2) - r_ceil))
    y1_b = min(H, int(max(y1, y2) + r_ceil + 1))
    if x0 >= x1_b or y0 >= y1_b:
        return

    dx = x2 - x1
    dy = y2 - y1
    l2 = dx*dx + dy*dy

    yy, xx = np.ogrid[y0:y1_b, x0:x1_b]
    if l2 < 1e-4:
        dist_sq = (xx - x1)**2 + (yy - y1)**2
    else:
        t_proj = np.clip(((xx - x1)*dx + (yy - y1)*dy) / l2, 0.0, 1.0)
        px = x1 + t_proj * dx
        py = y1 + t_proj * dy
        dist_sq = (xx - px)**2 + (yy - py)**2

    r_sq = radius**2
    r_out_sq = (radius + 1.2)**2
    mask = np.clip((r_out_sq - dist_sq) / (r_out_sq - r_sq + 1e-5), 0.0, 1.0)
    a = mask * alpha
    if not np.any(a > 0.01):
        return

    sub = canvas[y0:y1_b, x0:x1_b]
    prev_a = sub[:, :, 3]
    out_a = a + prev_a * (1.0 - a)
    safe_a = np.where(out_a > 0.001, out_a, 1.0)

    for c in range(3):
        sub[:, :, c] = (col[c] * a + sub[:, :, c] * prev_a * (1.0 - a)) / safe_a
    sub[:, :, 3] = out_a

def draw_triangle(canvas, p0, p1, p2, col, alpha=1.0):
    H, W = canvas.shape[:2]
    xs = [p0[0], p1[0], p2[0]]
    ys = [p0[1], p1[1], p2[1]]
    x0 = max(0, int(math.floor(min(xs))))
    x1_b = min(W, int(math.ceil(max(xs))) + 1)
    y0 = max(0, int(math.floor(min(ys))))
    y1_b = min(H, int(math.ceil(max(ys))) + 1)
    if x0 >= x1_b or y0 >= y1_b:
        return

    yy, xx = np.ogrid[y0:y1_b, x0:x1_b]
    e0 = (xx - p0[0]) * (p1[1] - p0[1]) - (yy - p0[1]) * (p1[0] - p0[0])
    e1 = (xx - p1[0]) * (p2[1] - p1[1]) - (yy - p1[1]) * (p2[0] - p1[0])
    e2 = (xx - p2[0]) * (p0[1] - p2[1]) - (yy - p2[1]) * (p0[0] - p2[0])

    mask = ((e0 >= 0) & (e1 >= 0) & (e2 >= 0)) | ((e0 <= 0) & (e1 <= 0) & (e2 <= 0))
    if not np.any(mask):
        return

    a = mask.astype(np.float32) * alpha
    sub = canvas[y0:y1_b, x0:x1_b]
    prev_a = sub[:, :, 3]
    out_a = a + prev_a * (1.0 - a)
    safe_a = np.where(out_a > 0.001, out_a, 1.0)

    for c in range(3):
        sub[:, :, c] = (col[c] * a + sub[:, :, c] * prev_a * (1.0 - a)) / safe_a
    sub[:, :, 3] = out_a


# ==============================================================================
# Artwork Engine: HD Masterpiece Mode (160x160 Canvas)
# ==============================================================================

C_SCALE_DARK = np.array([0.05, 0.20, 0.16], dtype=np.float32)
C_SCALE_MID  = np.array([0.09, 0.38, 0.30], dtype=np.float32)
C_SCALE_LGT  = np.array([0.17, 0.62, 0.50], dtype=np.float32)
C_SCALE_SPEC = np.array([0.40, 0.85, 0.72], dtype=np.float32)

C_GOLD_DARK  = np.array([0.42, 0.28, 0.10], dtype=np.float32)
C_GOLD_MID   = np.array([0.72, 0.52, 0.18], dtype=np.float32)
C_GOLD_LGT   = np.array([0.92, 0.73, 0.32], dtype=np.float32)
C_GOLD_SPEC  = np.array([1.00, 0.92, 0.60], dtype=np.float32)

C_EYE_AMBER  = np.array([1.00, 0.72, 0.05], dtype=np.float32)
C_EYE_CORE   = np.array([1.00, 0.95, 0.40], dtype=np.float32)
C_EYE_SLIT   = np.array([0.12, 0.05, 0.02], dtype=np.float32)

C_CLAW       = np.array([0.92, 0.90, 0.82], dtype=np.float32)
# A hurt head, build ticket 192: its seat crashed, failed, was rate limited or was stopped.
C_HURT_DARK  = np.array([0.35, 0.03, 0.04], dtype=np.float32)
C_HURT_MID   = np.array([0.72, 0.10, 0.10], dtype=np.float32)
C_HURT_LGT   = np.array([0.92, 0.22, 0.18], dtype=np.float32)
C_CLAW_SHAD  = np.array([0.45, 0.40, 0.32], dtype=np.float32)

def render_hd_dragon(t, head_states, fire_levels, pet_active=False):
    W, H = 160, 160
    canvas = np.zeros((H, W, 4), dtype=np.float32)

    # 1. Breathing motion
    breathe = math.sin(t * 2.2)
    chest_scale = 1.0 + 0.025 * breathe
    body_y = 110.0 - 1.5 * breathe

    # 2. Coiled Dragon Tail at base
    tail_pts = []
    for deg in range(0, 240, 12):
        rad = math.radians(deg)
        r_dist = 36 - deg * 0.09
        tx = 115 + math.cos(rad) * r_dist
        ty = 128 + math.sin(rad) * (r_dist * 0.55)
        tail_pts.append((tx, ty, deg))

    for tx, ty, deg in tail_pts:
        t_rad = max(3.5, 12.0 - deg * 0.04)
        sp_x = tx + math.cos(math.radians(deg) - 0.5) * (t_rad + 3.5)
        sp_y = ty + math.sin(math.radians(deg) - 0.5) * (t_rad + 3.5)
        draw_triangle(canvas, (tx, ty), (sp_x, sp_y), (tx - 3, ty - 2), C_GOLD_MID)
        draw_circle(canvas, tx, ty, t_rad, C_SCALE_MID)
        draw_circle(canvas, tx - 1, ty - 1, t_rad * 0.7, C_SCALE_LGT)

    # 3. Main Torso
    draw_circle(canvas, 80, body_y + 14, 26 * chest_scale, C_SCALE_DARK)
    draw_circle(canvas, 80, body_y + 12, 25 * chest_scale, C_SCALE_MID)
    draw_circle(canvas, 80, body_y + 2, 22 * chest_scale, C_SCALE_LGT)

    # Segmented Golden Chest Plates
    for i in range(5):
        py = body_y - 8 + i * 7.0
        pw = (16.0 - abs(i - 2) * 2.8) * chest_scale
        draw_capsule(canvas, 80 - pw, py, 80 + pw, py, 4.0, C_GOLD_MID)
        draw_capsule(canvas, 80 - pw + 2, py - 1, 80 + pw - 2, py - 1, 2.2, C_GOLD_SPEC)

    # 4. Forelegs and Claws
    for side in (-1, 1):
        leg_x = 80 + side * 29
        leg_y = body_y + 16
        draw_capsule(canvas, 80 + side * 20, body_y + 6, leg_x, leg_y, 9.0, C_SCALE_MID)
        draw_capsule(canvas, leg_x, leg_y, leg_x + side * 5, leg_y + 18, 7.0, C_SCALE_LGT)

        foot_x = leg_x + side * 5
        foot_y = leg_y + 18
        for c_idx in (-1, 0, 1):
            c_base_x = foot_x + c_idx * 5.0
            c_tip_x = c_base_x + c_idx * 4.5 + side * 2.5
            c_tip_y = foot_y + 7.5
            draw_triangle(canvas, (c_base_x - 1.5, foot_y), (c_base_x + 1.5, foot_y), (c_tip_x, c_tip_y), C_CLAW)
            draw_capsule(canvas, c_base_x, foot_y - 1, c_base_x, foot_y + 1, 2.0, C_CLAW_SHAD)

    # 5. Necks & Heads configuration
    head_configs = [
        (0, 'left',   66, 44,  52, -0.35, 0.0),
        (1, 'center', 80, 80,  32,  0.00, 1.8),
        (2, 'right',  94, 116, 54,  0.35, 3.6),
    ]

    head_positions = []
    for idx, name, bx, def_hx, def_hy, ang_bias, phase in head_configs:
        state = head_states[idx]
        f_lvl = fire_levels[idx]
        is_sleep = (state == 'sleeping')
        is_hurt  = (state == 'hurt')
        is_fire  = (state == 'firing' or f_lvl > 0.05) and not is_hurt

        sway_x = math.sin(t * 1.8 + phase) * 3.5
        sway_y = math.cos(t * 1.5 + phase) * 2.5
        by = body_y - (14 if idx == 1 else 10)

        hx = def_hx + sway_x + (-3 if idx==0 and is_sleep else (3 if idx==2 and is_sleep else 0))
        hy = def_hy + sway_y + (12 if is_sleep else 0) - (6 if is_fire else 0) + (16 if is_hurt else 0)
        ang = ang_bias + (0.25 if is_sleep and idx!=1 else (0.1 if is_sleep else 0)) + (-0.15 if is_fire else 0)
        # A hurt head hangs: dropped further than asleep and turned down towards the body.
        if is_hurt:
            ang += 0.6 if idx != 2 else -0.6

        head_positions.append((idx, name, bx, by, hx, hy, ang, state, is_sleep, is_fire, f_lvl))

    # Draw Necks
    for idx, name, bx, by, hx, hy, ang, state, is_sleep, is_fire, f_lvl in head_positions:
        steps = 14
        prev_x, prev_y = bx, by
        for s in range(1, steps + 1):
            u = s / steps
            ctrl_x = (bx + hx) * 0.5 + (0 if idx==1 else (-14 if idx==0 else 14))
            ctrl_y = (by + hy) * 0.5 - 6.0
            nx = (1 - u)**2 * bx + 2 * (1 - u) * u * ctrl_x + u**2 * hx
            ny = (1 - u)**2 * by + 2 * (1 - u) * u * ctrl_y + u**2 * hy
            rad = 11.5 - u * 4.0

            sp_dir = -1 if idx == 0 else (1 if idx == 2 else math.sin(s * 1.5))
            sp_x = nx + sp_dir * (rad + 3.0)
            sp_y = ny - 1.5
            draw_triangle(canvas, (nx, ny - 3), (nx, ny + 3), (sp_x, sp_y), C_GOLD_MID)
            draw_circle(canvas, sp_x, sp_y, 2.0, C_GOLD_SPEC)

            draw_capsule(canvas, prev_x, prev_y, nx, ny, rad, C_SCALE_MID)
            draw_capsule(canvas, prev_x, prev_y, nx, ny, rad * 0.6, C_SCALE_LGT)

            th_x = nx - sp_dir * (rad * 0.45)
            draw_circle(canvas, th_x, ny + 1.0, rad * 0.45, C_GOLD_MID)
            draw_circle(canvas, th_x, ny + 1.0, rad * 0.25, C_GOLD_SPEC)

            prev_x, prev_y = nx, ny

    # Draw Heads & Facial Expressions
    for idx, name, bx, by, hx, hy, ang, state, is_sleep, is_fire, f_lvl in head_positions:
        is_hurt = (state == 'hurt')
        scale_mid = C_HURT_MID if is_hurt else C_SCALE_MID
        scale_lgt = C_HURT_LGT if is_hurt else C_SCALE_LGT
        cos_a = math.cos(ang)
        sin_a = math.sin(ang)

        def rot(dx, dy):
            return hx + dx * cos_a - dy * sin_a, hy + dx * sin_a + dy * cos_a

        # Crown Horns sweeping backwards
        draw_capsule(canvas, rot(-6, -6)[0], rot(-6, -6)[1], rot(-11, -19)[0], rot(-11, -19)[1], 3.8, C_GOLD_MID)
        draw_capsule(canvas, rot(6, -6)[0],  rot(6, -6)[1],  rot(11, -19)[0],  rot(11, -19)[1],  3.8, C_GOLD_MID)
        draw_circle(canvas, rot(-11, -19)[0], rot(-11, -19)[1], 2.0, C_GOLD_SPEC)
        draw_circle(canvas, rot(11, -19)[0],  rot(11, -19)[1],  2.0, C_GOLD_SPEC)

        draw_capsule(canvas, rot(-5, -3)[0], rot(-5, -3)[1], rot(-15, -11)[0], rot(-15, -11)[1], 2.6, C_GOLD_DARK)
        draw_capsule(canvas, rot(5, -3)[0],  rot(5, -3)[1],  rot(15, -11)[0],  rot(15, -11)[1],  2.6, C_GOLD_DARK)

        # Skull
        draw_circle(canvas, hx, hy, 12.5, scale_mid)
        draw_circle(canvas, hx, hy - 2, 10.0, scale_lgt)

        # Snout
        snout_x, snout_y = rot(0, 8)
        draw_circle(canvas, snout_x, snout_y, 8.5, scale_mid)
        draw_circle(canvas, snout_x, snout_y - 1, 7.0, scale_lgt)

        # Lower Jaw
        jaw_drop = 8.0 if is_fire else (1.5 if is_sleep or is_hurt else 3.0)
        chin_x, chin_y = rot(0, 9 + jaw_drop)
        draw_capsule(canvas, rot(-5, 5)[0], rot(-5, 5)[1], chin_x, chin_y, 4.2, C_GOLD_MID)
        draw_capsule(canvas, rot(5, 5)[0],  rot(5, 5)[1],  chin_x, chin_y, 4.2, C_GOLD_MID)

        # Mouth interior & teeth
        if is_fire or jaw_drop > 4.0:
            maw_x, maw_y = rot(0, 9 + jaw_drop * 0.4)
            draw_circle(canvas, maw_x, maw_y, 5.5, np.array([1.0, 0.4, 0.1], dtype=np.float32))
            draw_circle(canvas, maw_x, maw_y, 3.2, np.array([1.0, 0.95, 0.5], dtype=np.float32))
            draw_triangle(canvas, rot(-4, 7), rot(-2, 7), rot(-3, 11), C_CLAW)
            draw_triangle(canvas, rot(2, 7),  rot(4, 7),  rot(3, 11),  C_CLAW)
        elif not is_sleep and not is_hurt and idx == 2:
            t_len = 5.0 + math.sin(t * 12.0) * 3.0
            tx1, ty1 = rot(2, 11)
            tx2, ty2 = rot(5, 11 + t_len)
            draw_capsule(canvas, tx1, ty1, tx2, ty2, 1.2, np.array([0.95, 0.25, 0.4], dtype=np.float32))

        # Eyes (Glowing Amber vs Closed)
        for e_side in (-1, 1):
            ex, ey = rot(e_side * 6.2, -1.0)
            if is_hurt:
                # Shut tight, a dark crease with no gold lid line.
                draw_capsule(canvas, ex - 3.2, ey + 0.5, ex + 3.2, ey + 0.5, 1.6, C_HURT_DARK)
            elif is_sleep:
                draw_capsule(canvas, ex - 3.2, ey + 0.5, ex + 3.2, ey + 0.5, 1.4, C_SCALE_DARK)
                draw_capsule(canvas, ex - 2.5, ey, ex + 2.5, ey, 0.9, C_GOLD_DARK)
            else:
                draw_circle(canvas, ex, ey, 3.4, C_EYE_AMBER)
                draw_circle(canvas, ex, ey, 2.0, C_EYE_CORE)
                draw_capsule(canvas, ex, ey - 2.6, ex, ey + 2.6, 0.9, C_EYE_SLIT)

        # Fire Breath Particles
        if is_fire or f_lvl > 0.05:
            flame_ox, flame_oy = rot(0, 11 + jaw_drop)
            vx = -sin_a * 0.2 + (0.0 if idx==1 else (-0.75 if idx==0 else 0.75))
            vy = cos_a * 0.4 - 0.75
            p_count = int(18 * max(f_lvl, 0.8 if is_fire else 0.0))

            for pi in range(p_count):
                u = ((t * 9.0 + pi * 0.33) % 3.0) / 3.0
                dist = 6.0 + u * 48.0
                wobble = math.sin(pi * 8.3 + t * 20.0) * (14.0 * u)
                px = flame_ox + vx * dist + wobble
                py = flame_oy + vy * dist - u * 16.0
                prad = 3.2 + u * 10.0

                if u < 0.28:
                    col1 = np.array([1.0, 1.0, 0.85], dtype=np.float32)
                    al = 0.95
                elif u < 0.65:
                    col1 = np.array([1.0, 0.60, 0.1], dtype=np.float32)
                    al = 0.80
                else:
                    col1 = np.array([0.75, 0.12, 0.05], dtype=np.float32)
                    al = 0.50 * (1.0 - (u - 0.65) / 0.35)

                draw_circle(canvas, px, py, prad, col1, alpha=al)

        # Sleep Bubbles 🫧 and Drifting Zzz
        if is_sleep:
            nos_x, nos_y = rot(-2.0, 9.5)
            b_cycle = (t * 1.3 + idx * 1.1) % 3.2
            if b_cycle < 2.4:
                b_ratio = b_cycle / 2.4
                bx = nos_x + math.sin(t * 3.5 + idx) * 3.5 - 4.5 * b_ratio
                by = nos_y - b_ratio * 28.0
                brad = 2.2 + b_ratio * 5.2
                draw_circle(canvas, bx, by, brad, np.array([0.5, 0.8, 1.0], dtype=np.float32), alpha=0.65)
                draw_circle(canvas, bx - brad * 0.35, by - brad * 0.35, brad * 0.3, np.array([1.0, 1.0, 1.0], dtype=np.float32), alpha=0.9)

            z_cycle = (t * 1.1 + idx * 0.9) % 3.8
            z_ratio = z_cycle / 3.8
            zx = hx + math.sin(t * 2.2 + idx) * 7.0 + (12 if idx==2 else -12)
            zy = hy - 14.0 - z_ratio * 34.0
            z_alpha = math.sin(z_ratio * math.pi) * 0.9
            if z_alpha > 0.08:
                z_sz = 3.0 + z_ratio * 2.8
                z_col = np.array([0.65, 0.85, 1.0], dtype=np.float32)
                draw_capsule(canvas, zx - z_sz, zy - z_sz, zx + z_sz, zy - z_sz, 1.1, z_col, alpha=z_alpha)
                draw_capsule(canvas, zx + z_sz, zy - z_sz, zx - z_sz, zy + z_sz, 1.1, z_col, alpha=z_alpha)
                draw_capsule(canvas, zx - z_sz, zy + z_sz, zx + z_sz, zy + z_sz, 1.1, z_col, alpha=z_alpha)

    # Pet Affection Hearts & Sparkles ✨💖
    if pet_active:
        for hi in range(5):
            h_cycle = (t * 1.8 + hi * 0.6) % 2.5
            h_u = h_cycle / 2.5
            hx = 45 + hi * 18 + math.sin(t * 3.0 + hi) * 6.0
            hy = 40 - h_u * 32.0
            h_al = math.sin(h_u * math.pi) * 0.9
            if h_al > 0.1:
                draw_circle(canvas, hx - 2.5, hy - 2, 3.2, np.array([1.0, 0.2, 0.5], dtype=np.float32), alpha=h_al)
                draw_circle(canvas, hx + 2.5, hy - 2, 3.2, np.array([1.0, 0.2, 0.5], dtype=np.float32), alpha=h_al)
                draw_triangle(canvas, (hx - 5.5, hy - 1), (hx + 5.5, hy - 1), (hx, hy + 6.0), np.array([1.0, 0.2, 0.5], dtype=np.float32), alpha=h_al)
                draw_circle(canvas, hx + 6, hy - 5, 1.5, np.array([1.0, 0.9, 0.4], dtype=np.float32), alpha=h_al)

    img_uint8 = np.clip(canvas * 255.0, 0, 255).astype(np.uint8)
    return img_uint8


# ==============================================================================
# Artwork Engine: Compact 20px Mode (24x24 Pixel Art Micro-Pet)
# ==============================================================================

def render_compact_dragon(t, head_states, fire_levels, pet_active=False):
    W, H = 24, 24
    grid = np.zeros((H, W, 4), dtype=np.uint8)

    P_DK = (15, 60, 48, 255)
    P_MD = (25, 115, 90, 255)
    P_LT = (45, 185, 140, 255)
    P_GD = (195, 140, 45, 255)
    P_EY = (255, 195, 10, 255)
    P_CW = (235, 230, 210, 255)
    P_FL = (255, 80, 10, 255)
    P_BU = (140, 210, 255, 220)
    P_ZZ = (160, 200, 255, 240)
    P_HT = (255, 60, 120, 255)
    P_RD = (110, 12, 18, 255)
    P_RL = (225, 45, 40, 255)
    P_RH = (175, 85, 60, 255)

    breathe = 1 if (math.sin(t * 2.5) > 0.2) else 0

    # Base Dragon Body & Coiled Tail
    for x in range(3, 21):
        grid[22, x] = P_DK
    for x in range(4, 20):
        grid[21, x] = P_MD
    for x in range(6, 18):
        grid[20, x] = P_LT

    # Claws
    grid[22, 5] = P_CW
    grid[22, 7] = P_CW
    grid[22, 16] = P_CW
    grid[22, 18] = P_CW

    # Torso & Golden Belly Plates
    for y in range(16 - breathe, 20 - breathe):
        for x in range(7, 17):
            grid[y, x] = P_MD
        for x in range(9, 15):
            grid[y, x] = P_LT
        grid[y, 11] = P_GD
        grid[y, 12] = P_GD

    # 3 Heads configurations: Left (x=4), Center (x=11), Right (x=18)
    configs = [
        (0, 4, 9, -1),
        (1, 11, 7, 0),
        (2, 18, 9, 1),
    ]

    for idx, hx, hy, neck_bias in configs:
        st = head_states[idx]
        fl = fire_levels[idx]
        is_sleep = (st == 'sleeping')
        is_hurt  = (st == 'hurt')
        is_fire  = (st == 'firing' or fl > 0.05) and not is_hurt

        sway = int(math.sin(t * 2.0 + idx * 1.8) * 1.2)
        cur_hx = hx + sway
        cur_hy = hy + (2 if is_sleep or is_hurt else (0 if not is_fire else -1)) - breathe
        # A hurt head tilts down: its outer column hangs a pixel below the rest (left for the
        # left and centre heads, right for the right one).
        outer = cur_hx + (1 if idx == 2 else -1)

        def drop(x):
            return 1 if is_hurt and x == outer else 0

        # Neck connection
        for ny in range(cur_hy + 3, 16 - breathe):
            ratio = (ny - (cur_hy + 3)) / max(1, (16 - breathe - (cur_hy + 3)))
            nx = int(cur_hx * (1 - ratio) + (11 + neck_bias * 3) * ratio)
            grid[ny, nx] = P_MD
            if nx + 1 < W:
                grid[ny, nx + 1] = P_LT
            grid[ny, max(0, nx - 1)] = P_GD

        # Horns
        horn = P_RH if is_hurt else P_GD
        for x in (cur_hx - 1, cur_hx + 1):
            grid[cur_hy - 2 + drop(x), x] = horn
            grid[cur_hy - 1 + drop(x), x] = horn

        # Skull & Snout
        for y in range(cur_hy, cur_hy + 3):
            for x in range(cur_hx - 1, cur_hx + 2):
                if 0 <= y + drop(x) < H and 0 <= x < W:
                    grid[y + drop(x), x] = P_RL if is_hurt else P_LT

        # Eyes & Expressions
        eye_y = cur_hy
        if is_hurt:
            # Shut: a dark red crease where each eye was.
            for x in (cur_hx - 1, cur_hx + 1):
                if 0 <= eye_y + drop(x) < H:
                    grid[eye_y + drop(x), x] = P_RD
        elif is_sleep:
            if 0 <= eye_y < H:
                grid[eye_y, cur_hx - 1] = P_DK
                grid[eye_y, cur_hx + 1] = P_DK
            # Sleep bubble
            b_u = (t * 1.5 + idx * 0.8) % 2.0
            by = cur_hy - 1 - int(b_u * 3)
            bx = cur_hx + (-1 if idx == 0 else 1)
            if 0 <= by < H and 0 <= bx < W:
                grid[by, bx] = P_BU
            # Zzz
            zy = cur_hy - 2 - int(b_u * 4)
            zx = cur_hx + (2 if idx == 2 else -2)
            if 0 <= zy < H and 0 <= zx < W and b_u > 0.5:
                grid[zy, zx] = P_ZZ
        else:
            if 0 <= eye_y < H:
                grid[eye_y, cur_hx - 1] = P_EY
                grid[eye_y, cur_hx + 1] = P_EY

        # Fire breath
        if is_fire:
            mouth_y = cur_hy + 3
            grid[mouth_y, cur_hx] = P_FL
            fx_dir = -1 if idx == 0 else (0 if idx == 1 else 1)
            for f_i in range(1, 4):
                fy = mouth_y - f_i
                fx = cur_hx + fx_dir * f_i
                if 0 <= fy < H and 0 <= fx < W:
                    grid[fy, fx] = P_FL
                    if 0 <= fx + 1 < W:
                        grid[fy, fx + 1] = (255, 190, 40, 255)

    # Petting hearts
    if pet_active:
        hy = int(3 - math.sin(t * 4.0) * 2)
        grid[hy, 11] = P_HT
        grid[hy - 1, 10] = P_HT
        grid[hy - 1, 12] = P_HT

    return grid


# ==============================================================================
# Hydra State Engine: Head State Machine & Interactive Loop
# ==============================================================================

class HydraPet:
    def __init__(self, mode="kitty", size="hd", fps=24, idle_timeout=7.0):
        self.mode = mode
        self.size = size
        self.fps = fps
        self.idle_timeout = idle_timeout

        # States for Left, Center, Right heads: 'awake', 'sleeping', 'firing'. Both renderers
        # also draw 'hurt', which only Hydra's cockpit frames ask for.
        self.head_states = ['awake', 'awake', 'awake']
        self.fire_levels = [0.0, 0.0, 0.0]

        # Timers
        self.last_input_time = time.time()
        self.last_random_sleep_time = time.time()
        self.random_sleep_interval = random.uniform(5.0, 9.0)

        self.pet_active_until = 0.0
        self.fire_active_until = 0.0
        self.running = True
        self.all_sleeping = False

        # Status text
        self.status_msg = "🐲 HydraPet is awake and watchful! (Press 'h' for controls)"
        self.status_expiry = time.time() + 4.0

    def set_status(self, msg, duration=3.5):
        self.status_msg = msg
        self.status_expiry = time.time() + duration

    def wake_all(self):
        self.head_states = ['awake', 'awake', 'awake']
        self.all_sleeping = False
        self.last_input_time = time.time()
        self.set_status("✨ All heads awakened and alert!")

    def sleep_all(self):
        self.head_states = ['sleeping', 'sleeping', 'sleeping']
        self.all_sleeping = True
        self.set_status("💤 Shhh... All dragon heads are asleep!")

    def toggle_head_sleep(self, idx):
        self.last_input_time = time.time()
        names = ["Left", "Center", "Right"]
        if self.head_states[idx] == 'sleeping':
            self.head_states[idx] = 'awake'
            self.set_status(f"👁️ {names[idx]} head woke up!")
        else:
            self.head_states[idx] = 'sleeping'
            self.set_status(f"🫧 {names[idx]} head dozed off...")

    def trigger_fire(self, target="all"):
        self.last_input_time = time.time()
        self.fire_active_until = time.time() + 1.6
        if target == "all":
            self.head_states = ['firing', 'firing', 'firing']
            self.set_status("🔥 ROAAAR! All three heads breathe fire!")
        else:
            idx = int(target)
            self.head_states[idx] = 'firing'
            names = ["Left", "Center", "Right"]
            self.set_status(f"🔥 {names[idx]} head breathes roaring flames!")

    def trigger_pet(self):
        self.last_input_time = time.time()
        self.pet_active_until = time.time() + 2.0
        for i in range(3):
            if self.head_states[i] != 'firing':
                self.head_states[i] = 'awake'
        self.set_status("💖 You gently pet the dragon! The hydra purrs happily ✨")

    def toggle_size(self):
        self.last_input_time = time.time()
        self.size = "20" if self.size == "hd" else "hd"
        mode_name = "20px Compact Mode" if self.size == "20" else "Masterpiece HD Mode"
        self.set_status(f"📐 Switched to {mode_name}!")
        sys.stdout.write("\x1b[2J")
        sys.stdout.flush()

    def update_ai(self, now):
        # 1. Fire expiration
        if now > self.fire_active_until:
            for i in range(3):
                if self.head_states[i] == 'firing':
                    self.head_states[i] = 'awake'
                self.fire_levels[i] = max(0.0, self.fire_levels[i] - 0.15)
        else:
            for i in range(3):
                if self.head_states[i] == 'firing':
                    self.fire_levels[i] = min(1.0, self.fire_levels[i] + 0.25)

        # 2. Global Idle detection: "que si no se esta haciendo nada se duerman todas"
        idle_duration = now - self.last_input_time
        if idle_duration > self.idle_timeout and not self.all_sleeping:
            self.sleep_all()
            self.set_status("💤 The dragon drifted into deep slumber from inactivity...")

        # 3. Random single head sleep: "aleatoriamente una de las cabezas se duerma, cierre ojos, escupa burbujitas y zzz"
        if not self.all_sleeping and (now - self.last_random_sleep_time) > self.random_sleep_interval:
            self.last_random_sleep_time = now
            self.random_sleep_interval = random.uniform(6.0, 12.0)

            awake_heads = [i for i in range(3) if self.head_states[i] == 'awake']
            sleeping_heads = [i for i in range(3) if self.head_states[i] == 'sleeping']

            if awake_heads and len(sleeping_heads) < 2:
                idx = random.choice(awake_heads)
                self.head_states[idx] = 'sleeping'
                names = ["Left", "Center", "Right"]
                self.set_status(f"🫧 {names[idx]} head got sleepy and dozed off...")
            elif sleeping_heads:
                idx = random.choice(sleeping_heads)
                self.head_states[idx] = 'awake'
                names = ["Left", "Center", "Right"]
                self.set_status(f"🥱 {names[idx]} head yawned and woke up!")

    def render_frame(self, t):
        now = time.time()
        pet_active = (now < self.pet_active_until)

        # Generate raster image
        if self.size == "hd" and HAS_NUMPY:
            img = render_hd_dragon(t, self.head_states, self.fire_levels, pet_active)
            cols, rows = 44, 22
        else:
            img = render_compact_dragon(t, self.head_states, self.fire_levels, pet_active)
            cols, rows = 24, 12

        # Output to terminal
        buf = []
        buf.append("\x1b[1;1H")

        if self.mode == "kitty":
            png_bytes = encode_png(img.shape[1], img.shape[0], img.tobytes())
            buf.append(render_kitty_chunks(png_bytes, cols=cols, rows=rows, image_id=1))
        else:
            buf.append(render_ansi_halfblocks(img, cols=cols, rows=rows))

        # Status bar & controls
        status_row = rows + 2
        buf.append(f"\x1b[{status_row};1H\x1b[K")

        badges = []
        names = ["Left", "Center", "Right"]
        for idx in range(3):
            st = self.head_states[idx]
            if st == 'firing':
                badge = f"\x1b[1;31m[{names[idx]}: 🔥 FIRE]\x1b[0m"
            elif st == 'sleeping':
                badge = f"\x1b[1;36m[{names[idx]}: 💤 SLEEP]\x1b[0m"
            else:
                badge = f"\x1b[1;32m[{names[idx]}: 👁️ AWAKE]\x1b[0m"
            badges.append(badge)

        buf.append(" ".join(badges) + f"  \x1b[90m(Mode: {self.size.upper()})\x1b[0m\n\x1b[K")
        buf.append(f"\x1b[1;33m{self.status_msg}\x1b[0m\n\x1b[K")
        buf.append(f"\x1b[90mControls: [Space/F] Fire  [P] Pet  [1/2/3] Toggle Head  [S] Sleep/Wake  [Tab] Size  [Q] Quit\x1b[0m\x1b[K")

        sys.stdout.write("".join(buf))
        sys.stdout.flush()

    def run(self):
        TerminalEnv.hide_cursor()
        TerminalEnv.enter_alt_screen()

        import tty, termios
        fd = sys.stdin.fileno()
        old_settings = termios.tcgetattr(fd)

        def sig_handler(signum, frame):
            self.running = False

        signal.signal(signal.SIGINT, sig_handler)
        signal.signal(signal.SIGTERM, sig_handler)

        start_time = time.time()
        frame_interval = 1.0 / self.fps

        try:
            tty.setcbreak(fd)
            while self.running:
                frame_start = time.time()
                t = frame_start - start_time

                # Check for keyboard inputs
                while True:
                    r, _, _ = select.select([sys.stdin], [], [], 0)
                    if not r:
                        break
                    ch = sys.stdin.read(1)
                    if ch in ('q', 'Q', '\x03'):
                        self.running = False
                        break
                    elif ch in (' ', 'f', 'F'):
                        self.trigger_fire("all")
                    elif ch in ('1', '2', '3'):
                        self.toggle_head_sleep(int(ch) - 1)
                    elif ch in ('p', 'P'):
                        self.trigger_pet()
                    elif ch in ('s', 'S'):
                        if self.all_sleeping or all(s == 'sleeping' for s in self.head_states):
                            self.wake_all()
                        else:
                            self.sleep_all()
                    elif ch in ('\t', 'm', 'M'):
                        self.toggle_size()
                    else:
                        if self.all_sleeping:
                            self.wake_all()
                        else:
                            self.last_input_time = time.time()

                self.update_ai(frame_start)
                self.render_frame(t)

                elapsed = time.time() - frame_start
                sleep_time = max(0.002, frame_interval - elapsed)
                time.sleep(sleep_time)

        finally:
            termios.tcsetattr(fd, termios.TCSADRAIN, old_settings)
            TerminalEnv.leave_alt_screen()
            print("\n🐉 HydraPet safely returned to the dragon lair. Fare thee well!\n")


# ==============================================================================
# CLI Entrypoint
# ==============================================================================

def main():
    import argparse
    parser = argparse.ArgumentParser(
        description="🐉 HydraPet: Animated 3-Headed Emerald Dragon Pet for Ghostty & Modern Terminals"
    )
    parser.add_argument(
        "--mode", choices=["auto", "kitty", "ansi"], default="auto",
        help="Rendering protocol: 'kitty' (default for Ghostty/Kitty), 'ansi' (TrueColor half-blocks), or 'auto'"
    )
    parser.add_argument(
        "--size", choices=["hd", "20"], default="hd",
        help="Initial pet size: 'hd' (160x160 masterpiece) or '20' (24x24 compact pixel-art)"
    )
    parser.add_argument(
        "--fps", type=int, default=24,
        help="Target frame rate (default: 24 FPS)"
    )
    parser.add_argument(
        "--idle", type=float, default=7.0,
        help="Inactivity seconds before all 3 heads fall asleep (default: 7.0s)"
    )

    args = parser.parse_args()

    protocol = args.mode
    if protocol == "auto":
        protocol = "kitty" if TerminalEnv.supports_kitty() else "ansi"

    pet = HydraPet(mode=protocol, size=args.size, fps=args.fps, idle_timeout=args.idle)
    pet.run()

if __name__ == "__main__":
    main()
