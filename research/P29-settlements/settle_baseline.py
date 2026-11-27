"""Per-seed baseline of theos_maps 0.3.0 settlement child runs (P29 probe, CE-78.3).

  python scripts/settle_baseline.py <parent_engine_dir> <feature_id> <out_dir> [--seeds 1..8] [--tag T]

Calls theos_maps.run.run_one with SETTLEMENT_RECIPE and the child seed set directly (details.py
would derive crc32("<parent_seed>:<feature>") % 100000 instead; that seed is recorded too).
Run with PYTHONPATH=<toolset copy>, MPLBACKEND=Agg.
"""
import argparse, hashlib, json, shutil, time
from pathlib import Path
import numpy as np
from scipy import ndimage
from skimage.draw import polygon as draw_polygon

from theos_maps import core
from theos_maps.details import SETTLEMENT_RECIPE, child_seed
from theos_maps.run import run_one
from theos_maps.settlement import CLASS, LANDMARKS


def rings_mask(shape, ring):
    rr, cc = draw_polygon([q[0] for q in ring], [q[1] for q in ring], shape)
    return rr, cc


def wall_line(shape, wall):
    ny, nx = shape
    line = np.zeros(shape, bool)
    for (y1, x1), (y2, x2) in zip(wall, wall[1:]):
        n_ = int(max(abs(y2 - y1), abs(x2 - x1))) + 1
        ly = np.clip(np.linspace(y1, y2, n_).astype(int), 0, ny - 1)
        lx = np.clip(np.linspace(x1, x2, n_).astype(int), 0, nx - 1)
        line[ly, lx] = True
    return line


def extra_metrics(w):
    cell, shape = w["cell_m"], w["shape"]
    cls = w["settlement_class"]
    C = CLASS[cls]
    gy, gx = np.gradient(w["z"], cell)
    slope = np.hypot(gy, gx)
    water = w["river"] | w["lake"]
    sc = w["street_class"]
    d_street = ndimage.distance_transform_edt(sc == 0) * cell      # metres to nearest street cell
    main_half = 2 if cell <= 3 else 1
    street_w_m = (2 * main_half + 1) * cell                          # widest stamped street
    plot_depth_m = C["size"][1] * 0.9                                 # deepest plot
    thr = street_w_m + plot_depth_m
    acc, gaps, on_slope, on_water, on_street, overlap = 0, [], 0, 0, 0, 0
    occ = np.zeros(shape, np.int16)
    for b in w["buildings"]:
        rr, cc = rings_mask(shape, b["ring"])
        if len(rr) == 0:
            continue
        cy, cx = rr.mean(), cc.mean()
        dc = d_street[int(round(cy)), int(round(cx))]
        acc += dc <= thr
        gaps.append(float(d_street[rr, cc].min()))
        on_slope += bool((slope[rr, cc] > 0.3).any())
        on_water += bool(water[rr, cc].any())
        on_street += bool((sc[rr, cc] > 0).any())
        occ[rr, cc] += 1
    overlap_cells = int((occ > 1).sum())
    lab, ncomp = ndimage.label(sc > 0, structure=np.ones((3, 3)))
    out = {
        "street_access_threshold_m": round(thr, 1),
        "street_access_share": round(acc / max(1, len(w["buildings"])), 4),
        "building_street_gap_m": {"max": round(max(gaps), 1) if gaps else None,
                                   "p95": round(float(np.percentile(gaps, 95)), 1) if gaps else None},
        "buildings_on_slope_gt_0_3": int(on_slope),
        "buildings_on_water": int(on_water),
        "buildings_on_street": int(on_street),
        "building_overlap_cells": overlap_cells,
        "street_components": int(ncomp),
        "landmarks_expected": LANDMARKS[cls],
        "landmarks_missing": sorted(set(LANDMARKS[cls]) - {l["type"] for l in w["landmarks"]}),
        "wall": bool(w.get("wall")),
        "gates": len(w.get("gates") or []),
    }
    if w.get("wall"):
        line = wall_line(shape, w["wall"])
        out["wall_lake_cells"] = int((line & w["lake"]).sum())
        out["wall_lake_crossings"] = int(ndimage.label(line & w["lake"], structure=np.ones((3, 3)))[1])
        out["wall_river_crossings"] = int(ndimage.label(line & w["river"], structure=np.ones((3, 3)))[1])
        out["wall_length_m"] = round(float(sum(np.hypot(y2 - y1, x2 - x1) for (y1, x1), (y2, x2)
                                               in zip(w["wall"], w["wall"][1:])) * cell), 1)
    else:
        out["wall_lake_cells"] = out["wall_lake_crossings"] = out["wall_river_crossings"] = 0
    return out


def world_hash(w):
    h = hashlib.sha256()
    for k in ("z", "river", "lake", "street_class", "building_mask", "field_type"):
        h.update(np.ascontiguousarray(w[k]).tobytes())
    h.update(json.dumps([w["streets"], w["buildings"], w["landmarks"], w.get("wall"), w.get("gates")],
                        default=float, sort_keys=True).encode())
    return h.hexdigest()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("parent"); ap.add_argument("feature"); ap.add_argument("out")
    ap.add_argument("--seeds", type=int, nargs="+", default=list(range(1, 9)))
    ap.add_argument("--tag", default="")
    a = ap.parse_args()
    parent = Path(a.parent).resolve()
    man = json.loads((parent / "manifest.json").read_text())
    feat = next(f for f in json.loads((parent / "features.json").read_text())["features"] if f["id"] == a.feature)
    name = f"{man['provenance']['recipe']['name'] if 'provenance' in man else parent.name}-{a.feature}"
    recipe = {**SETTLEMENT_RECIPE, "name": name, "parent": {"package": str(parent), "feature": a.feature}}
    out = Path(a.out); out.mkdir(parents=True, exist_ok=True)
    renders = out / "renders"; renders.mkdir(exist_ok=True)
    pl = {"package": parent.name, "feature": a.feature, "kind": feat["properties"]["type"],
          "anchor_m": feat["geometry"]["coordinates"]}
    rows = []
    for s in a.seeds:
        t0 = time.perf_counter()
        stem = out / "raw" / f"{name}_s{s}{a.tag}"
        stem.parent.mkdir(exist_ok=True)
        try:
            w, images, _ = run_one(recipe, s, stem, (), pl, do_export=False)
        except Exception as e:
            rows.append({"seed": s, "error": f"{type(e).__name__}: {e}", "run_s": round(time.perf_counter() - t0, 2)})
            continue
        run_s = round(time.perf_counter() - t0, 2)
        sm = {k: v for k, v in w.summary.items() if k != "pipeline"}
        png = renders / f"{name}_s{s}{a.tag}_plan.png"
        shutil.copyfile(images[0], png)
        row = {"seed": s, "run_s": run_s, **sm, **extra_metrics(w),
               "world_sha256": world_hash(w),
               "plan_png": str(png), "plan_png_sha256": hashlib.sha256(png.read_bytes()).hexdigest()}
        rows.append(row)
        print(json.dumps({k: row[k] for k in ("seed", "run_s", "buildings", "houses", "gates", "street_access_share",
                                                "buildings_on_slope_gt_0_3", "buildings_on_water")}))
    meta = {"parent": str(parent), "parent_seed": man.get("seed"), "feature": a.feature,
            "class": feat["properties"]["type"], "name": feat["properties"].get("name"),
            "details_py_child_seed": child_seed(man.get("seed"), a.feature), "seeds": a.seeds, "runs": rows}
    (out / f"{name}{a.tag}_results.json").write_text(json.dumps(meta, indent=1, default=float))


if __name__ == "__main__":
    main()
