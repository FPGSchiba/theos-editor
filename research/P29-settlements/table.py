import json, sys, glob
from pathlib import Path
root = Path(sys.argv[1]); lines = []; allr = {}
H = "| map | seed | s | bldg | houses | pop | km main/street/lane | landmarks | miss | gates | bridges | ha | wall | access | slope>0.3 | on water | on street | overlap cells | wall lake x | wall river x | street comps |"
for f in sorted(root.glob("*/*_results.json")):
    if "_rerun" in f.name: continue
    d = json.loads(f.read_text()); key = f.stem.replace("_results", "")
    allr[key] = d
    lines.append(f"\n**{key}** ({d['class']} {d['name']}; details.py child seed would be {d['details_py_child_seed']})\n\n{H}\n|" + "---|" * H.count("|")[:0].__len__() if False else f"\n**{key}** ({d['class']} {d['name']}; details.py crc32 child seed = {d['details_py_child_seed']})\n\n{H}\n" + "|---" * (H.count("|") - 1) + "|")
    for r in d["runs"]:
        if "error" in r:
            lines.append(f"| {key} | {r['seed']} | ERROR {r['error']} |"); continue
        km = r.get("street_km", {})
        lines.append(f"| {d['feature']} | {r['seed']} | {r['run_s']} | {r['buildings']} | {r['houses']} | {r['population_est']} | "
                     f"{km.get('main',0)}/{km.get('street',0)}/{km.get('lane',0)} | {len(r['landmarks'])}: {','.join(r['landmarks'])} | "
                     f"{','.join(r['landmarks_missing']) or '-'} | {r['gates']} | {r['bridges']} | {r['built_area_ha']} | {'y' if r['wall'] else 'n'} | "
                     f"{r['street_access_share']} | {r['buildings_on_slope_gt_0_3']} | {r['buildings_on_water']} | {r['buildings_on_street']} | "
                     f"{r['building_overlap_cells']} | {r['wall_lake_crossings']} | {r['wall_river_crossings']} | {r['street_components']} |")
# determinism
det = ["\n## Determinism (seed 1, second process)\n"]
for f in sorted(root.glob("*/*_rerun_results.json")):
    a = json.loads(f.read_text())["runs"][0]
    b = json.loads(Path(str(f).replace("_rerun", "")).read_text())["runs"][0]
    ign = {"run_s", "plan_png"}
    diff = [k for k in set(a) | set(b) if k not in ign and a.get(k) != b.get(k)]
    det.append(f"- {f.stem}: world_sha256 equal={a['world_sha256']==b['world_sha256']}, png_sha256 equal={a['plan_png_sha256']==b['plan_png_sha256']}, differing keys={diff}")
out = "# theos_maps 0.3.0 settlement baseline, seeds 1-8\n" + "\n".join(lines) + "\n" + "\n".join(det) + "\n"
(root / "baseline_table.md").write_text(out)
(root / "baseline_all.json").write_text(json.dumps(allr, indent=1))
print(out)
