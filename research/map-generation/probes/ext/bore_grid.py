"""Probe extension (research stage 4, not part of theos_maps): a grid stage with fixed
base-level cells INSIDE the map (the Bore), which the core `grid` stage cannot express,
and a check stage that counts what the unmodified core stages then put into the hole.

fastscapelib 0.3.0 supports this directly: RasterGrid.from_length(..., nodes_status={(row, col): FIXED_VALUE}).
"""
import numpy as np
import fastscapelib as fs
from theos_maps.core import stage
from theos_maps.common import spectral_noise


@stage("grid_bore", provides=("shape", "dx", "dy", "fs_grid", "fixed", "bore"), kind="terrain")
def grid_bore(w, p):
    """Raster domain with drained edges AND a drained hole. params: edges (may be empty),
    bore{at [y,x] normalised, r (fraction of width), ragged (0..1)}"""
    ny, nx = w.recipe["grid"]
    ly, lx = (v * 1000 for v in w.recipe["size_km"])
    S = fs.NodeStatus
    side = {"west": 0, "east": 1, "north": 2, "south": 3}
    bounds = [S.FIXED_GRADIENT] * 4
    for e in p.get("edges", []):
        bounds[side[e]] = S.FIXED_VALUE
    b = p.get("bore", {})
    cy, cx = b.get("at", [0.5, 0.5])
    yy, xx = np.mgrid[0:ny, 0:nx]
    d = np.hypot(yy / ny - cy, xx / nx - cx) / b.get("r", 0.12)
    d = d + b.get("ragged", 0.15) * spectral_noise((ny, nx), w.subseed("bore"), 2.6)
    bore = d < 1
    bore[0, :] = bore[-1, :] = bore[:, 0] = bore[:, -1] = False
    nodes = {(int(y), int(x)): S.FIXED_VALUE for y, x in np.argwhere(bore)}
    g = fs.RasterGrid.from_length([ny, nx], [ly, lx], bounds, nodes)
    w["fs_grid"], w["shape"] = g, (ny, nx)
    w["dx"], w["dy"] = lx / nx, ly / ny
    w["fixed"] = np.asarray(g.nodes_status()).reshape(ny, nx) == int(S.FIXED_VALUE)
    w["bore"] = bore
    w.summary["bore_pct"] = round(float(bore.mean() * 100), 2)


@stage("bore_check", needs=("bore", "z"), kind="terrain")
def bore_check(w, p):
    """Counts features, road cells, lake and river cells the core stages placed inside the
    Bore, measures the rim, then sinks the hole so the truth render shows it. params: depth_m"""
    bore = w["bore"]
    inside = [f["id"] for f in w.get("features", []) if bore[tuple(f["yx"])]]
    from scipy import ndimage
    rim = ndimage.binary_dilation(bore, iterations=2) & ~bore
    w.summary["probe_extra"] = {
        "features_inside_bore": inside,
        "road_cells_inside_bore": int((w.get("road_class", np.zeros(bore.shape)) > 0)[bore].sum()),
        "lake_cells_inside_bore": int(w["lake"][bore].sum()) if "lake" in w else None,
        "river_cells_inside_bore": int(w["river"][bore].sum()) if "river" in w else None,
        "rim_height_m": {"min": round(float(w["z"][rim].min()), 1), "median": round(float(np.median(w["z"][rim])), 1),
                         "max": round(float(w["z"][rim].max()), 1)},
        "streams_reaching_rim": int(ndimage.label(ndimage.binary_dilation(bore, iterations=1) & ~bore & w["river"])[1]) if "river" in w else None,
    }
    w["z"] = np.where(bore, w["z"].min() - p.get("depth_m", 300), w["z"])
