"""Dev tool: validate content/characters.toml and render every frame to a PNG contact sheet.
Usage: python3 tools/preview_sprites.py out.png"""
import sys, tomllib, os
os.environ.setdefault("PYGAME_HIDE_SUPPORT_PROMPT", "1")
import pygame

FRAMES = ["idle1", "idle2", "happy", "sad", "eat", "sleep"]

def grid(text):
    rows = [r for r in text.strip("\n").split("\n")]
    return rows

def resolve(ch, name, seen=()):
    f = ch[name]
    if isinstance(f, str):
        rows = grid(f)
    else:
        base = resolve(ch, f["base"], seen + (name,))
        rows = list(base)
        sx, sy = f.get("shift_x", 0), f.get("shift_y", 0)
        if sx or sy:
            new = ["." * 16 for _ in range(16)]
            new = [list(r) for r in new]
            for y, r in enumerate(rows):
                for x, c in enumerate(r):
                    if c == "#" and 0 <= x + sx < 16 and 0 <= y + sy < 16:
                        new[y + sy][x + sx] = "#"
            rows = ["".join(r) for r in new]
        for k, v in f.get("rows", {}).items():
            rows[int(k)] = v
    return rows

def main(out):
    data = tomllib.load(open(os.path.join(os.path.dirname(__file__), "..", "content", "characters.toml"), "rb"))
    chars = data["character"]
    errs = 0
    S = 6
    surf = pygame.Surface((len(FRAMES) * 18 * S + 160, len(chars) * 18 * S))
    surf.fill((178, 190, 166))
    pygame.font.init(); font = pygame.font.Font(None, 26)
    for r, ch in enumerate(chars):
        surf.blit(font.render(ch["name"], True, (0, 0, 0)), (len(FRAMES) * 18 * S + 6, r * 18 * S + 40))
        for c, fr in enumerate(FRAMES):
            rows = resolve(ch, fr)
            if len(rows) != 16 or any(len(x) != 16 for x in rows):
                bad = [(i, len(x)) for i, x in enumerate(rows) if len(x) != 16]
                print("BAD", ch["id"], fr, "rows=%d" % len(rows), bad)
                errs += 1
            for y, row in enumerate(rows):
                for x, cc in enumerate(row[:16]):
                    col = (32, 38, 36) if cc == "#" else (168, 180, 157)
                    surf.fill(col, ((c * 18 + 1 + x) * S, (r * 18 + 1 + y) * S, S - 1, S - 1))
    pygame.image.save(surf, out)
    print("characters:", len(chars), "errors:", errs)

main(sys.argv[1])
