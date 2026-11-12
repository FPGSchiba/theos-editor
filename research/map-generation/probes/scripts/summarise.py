"""Compact table from <name>_results.json files:  python scripts/summarise.py <results.json> ..."""
import json, sys
def g(d, path):
    for p in path.split("."):
        d = d.get(p) if isinstance(d, dict) else None
    return d
for f in sys.argv[1:]:
    for r in json.load(open(f)):
        if "error" in r:
            print(r["seed"], "ERROR", r["error"]); continue
        keys = ["seed", "run_s", "elevation_m.relief", "lake_pct", "rivers_pct", "max_discharge_m3s",
                "vegetation_cover_pct", "probe.slope_deg", "probe.slope_pct_over_45deg",
                "probe.towers_100m_drop_within_150m", "probe.local_peaks", "feature_counts", "roads", "sites",
                "travel.extraction_to_extraction_min", "topology", "analysis", "cover_items",
                "cls", "window_m", "cell_m", "grid_cells", "buildings", "population_est", "street_km", "gates",
                "bridges", "landmarks", "storeys_hist", "storeys_max", "street_grid_index", "wall", "regional_roads_entering", "built_area_ha", "probe_extra"]
        row = {k: g(r, k) for k in keys if g(r, k) is not None}
        c = r.get("constraints", {})
        row["constraints_failed"] = [k for k, v in c.items() if isinstance(v, dict) and not v["ok"]]
        row["all_ok"] = c.get("all_ok")
        print(json.dumps(row, default=float))
