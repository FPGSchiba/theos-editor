# theos-maps

Library. The deterministic terrain generator for maps, built on the map package format. Every
step of generation is a `Stage` (`stage`): it declares the world layers it reads and writes for
its parameters, runs in a `StageContext` that hands out only those layers, and is one line in
`stages::register`. The crate also holds the raster `Grid` with its D8/D16 neighbour tables, the
float and cast helpers (`kernels::num`), the per-stage random streams (`rng`) and the run's
progress events and cancel token (`progress`).
