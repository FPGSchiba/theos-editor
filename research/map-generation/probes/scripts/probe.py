"""Probe driver for theos_maps 0.3.0 (research stage 4).

  python scripts/probe.py <recipe.yaml> <out_dir> [--seeds 1 2 3 4] [--ext DIR] [--export]
                          [--sheet NAME.png] [--cols 2] [--tile 560]

Runs the recipe per seed through theos_maps.run.run_one, adds probe metrics (slope
distribution, tower count) to each summary, writes <name>_results.json and a contact sheet
with the caveat banner. Run from probes/ with PYTHONPATH=toolset.
"""
import argparse, json, sys, time, traceback
from pathlib import Path
import numpy as np

CAVEAT = ("CAVEAT: theos_maps 0.3.0 has no sediment deposition and no multiple-flow routing (the planned Rust "
          "generator adds both). A probe shows what the architecture and recipe model can express, "
          "not what the new generator's maps will look like.")


def terrain_metrics(w):
    from scipy import ndimage
    z, dx = w["z"], w["dx"]
    s = np.degrees(np.arctan(w["slope"])) if "slope" in w else None
    out = {}
    if s is not None:
        out["slope_deg"] = {f"p{q}": round(float(np.percentile(s, q)), 1) for q in (50, 90, 99)}
        out["slope_deg"]["max"] = round(float(s.max()), 1)
        out["slope_pct_over_30deg"] = round(float((s > 30).mean() * 100), 2)
        out["slope_pct_over_45deg"] = round(float((s > 45).mean() * 100), 2)
        out["slope_pct_over_60deg"] = round(float((s > 60).mean() * 100), 2)
    # isolated towers: local maxima that stand >= 100 m above the ground 150 m away in all 8 directions
    r = max(1, int(round(150 / dx)))
    peak = (z == ndimage.maximum_filter(z, size=2 * r + 1))
    ok = peak.copy()
    for oy, ox in [(0, 1), (1, 0), (1, 1), (1, -1), (0, -1), (-1, 0), (-1, -1), (-1, 1)]:
        sh = np.roll(z, (oy * r, ox * r), (0, 1))
        ok &= (z - sh) >= 100
    ok[:r, :] = ok[-r:, :] = ok[:, :r] = ok[:, -r:] = False
    peak[:r, :] = peak[-r:, :] = peak[:, :r] = peak[:, -r:] = False
    out["local_peaks"] = int(peak.sum())
    out["towers_100m_drop_within_150m"] = int(ok.sum())
    out["cell_m"] = round(float(dx), 1)
    return out


def street_grid_index(w, win_m=40.0):
    """Mean resultant length of 4*theta over street chords of ~win_m: 0 = directions spread
    evenly (organic), 1 = every street runs along one of two perpendicular axes (a grid)."""
    k = max(2, int(win_m / w["cell_m"]))
    ang = []
    for s in w["streets"]:
        c = np.asarray(s["cells"], float)
        for i in range(0, len(c) - k, k):
            d = c[i + k] - c[i]
            ang.append(np.arctan2(d[0], d[1]))
    if not ang:
        return None
    a = 4 * np.asarray(ang)
    return round(float(np.hypot(np.cos(a).mean(), np.sin(a).mean())), 3)


def banner_sheet(tiles, labels, out_path, cols=2, tile_w=560, title=""):
    from PIL import Image, ImageDraw, ImageFont
    import textwrap
    ims = []
    for p in tiles:
        im = Image.open(p).convert("RGB")
        ims.append(im.resize((tile_w, int(im.height * tile_w / im.width)), Image.LANCZOS))
    th = max(i.height for i in ims)
    rows = (len(ims) + cols - 1) // cols
    W = cols * tile_w
    try:
        font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 13)
    except OSError:
        font = ImageFont.load_default()
    lines = ([title] if title else []) + textwrap.wrap(CAVEAT, width=max(40, W // 8))
    bh = 18 * len(lines) + 10
    sheet = Image.new("RGB", (W, bh + rows * th), "white")
    d = ImageDraw.Draw(sheet)
    for i, l in enumerate(lines):
        d.text((6, 5 + 18 * i), l, fill=(150, 0, 0) if (i or not title) else "black", font=font)
    for i, t in enumerate(ims):
        x, y = (i % cols) * tile_w, bh + (i // cols) * th
        sheet.paste(t, (x, y))
        if labels:
            d.rectangle([x, y, x + 8 * len(labels[i]) + 10, y + 19], fill="white")
            d.text((x + 4, y + 2), labels[i], fill="black", font=font)
    sheet.save(out_path, optimize=True)
    if Path(out_path).stat().st_size > 2_000_000:      # keep sheets small: 256-colour palette
        sheet.quantize(256, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE).save(out_path, optimize=True)
    if Path(out_path).stat().st_size > 2_000_000:
        sheet.resize((W * 3 // 4, sheet.height * 3 // 4), Image.LANCZOS).quantize(256).save(out_path, optimize=True)
    return Path(out_path).stat().st_size


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("recipe"); ap.add_argument("out")
    ap.add_argument("--seeds", type=int, nargs="+", default=[1, 2, 3, 4])
    ap.add_argument("--ext", nargs="*", default=[])
    ap.add_argument("--export", action="store_true")
    ap.add_argument("--sheet"); ap.add_argument("--cols", type=int, default=2); ap.add_argument("--tile", type=int, default=560)
    ap.add_argument("--render-index", type=int, default=0)
    ap.add_argument("--parent-template", help="settlement recipes: parent package path with {seed}")
    a = ap.parse_args()
    print(CAVEAT)
    from theos_maps import core
    from theos_maps.run import run_one
    loaded = core.load_extensions(a.ext)
    recipe = core.load_recipe(a.recipe)
    out = Path(a.out); out.mkdir(parents=True, exist_ok=True)
    results, tiles, labels = [], [], []
    for s in a.seeds:
        t0 = time.time()
        if a.parent_template:
            recipe = {**recipe, "parent": {**recipe["parent"], "package": a.parent_template.format(seed=s)}}
        try:
            w, images, _ = run_one(recipe, s, out / f"{recipe['name']}_s{s}", loaded, recipe.get("parent"), a.export)
        except Exception as e:
            traceback.print_exc()
            results.append({"seed": s, "error": f"{type(e).__name__}: {e}", "run_s": round(time.time() - t0, 1)})
            continue
        sm = dict(w.summary)
        sm["run_s"] = round(time.time() - t0, 1)
        if recipe["kind"] == "terrain":
            sm["probe"] = terrain_metrics(w)
        if recipe["kind"] == "settlement":
            st = [b["storeys"] for b in w["buildings"]]
            sm["storeys_hist"] = {str(k): st.count(k) for k in sorted(set(st))}
            sm["grid_cells"] = list(w["shape"])
            sm["storeys_max"] = max(st) if st else 0
            sm["street_grid_index"] = street_grid_index(w)
            sm["wall"] = bool(w.get("wall"))
        sm.pop("poi", None); sm.pop("pipeline", None)
        results.append(sm)
        tiles.append(images[a.render_index]); labels.append(f"seed {s}")
        print(json.dumps(sm, default=float))
    (out / f"{recipe['name']}_results.json").write_text(json.dumps(results, indent=1, default=float))
    if a.sheet and tiles:
        size = banner_sheet(tiles, labels, a.sheet, a.cols, a.tile, title=f"{recipe['name']} - theos_maps 0.3.0 probe")
        print("sheet", a.sheet, size, "bytes")


if __name__ == "__main__":
    sys.exit(main())
