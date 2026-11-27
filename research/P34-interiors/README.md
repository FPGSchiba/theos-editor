# P34 interiors — probe

A throwaway probe for the P34 phase spec (`docs/specs/phases/P34-interiors.md` §4.3): cyclic
mission graphs on a coarse lattice, in the manner of Unexplored (Dormans 2017; Boris the Brave
2021), with two space steps (`sparse`: rooms inside lattice cells joined by corridors; `dense`:
rooms fill their cells and share walls with doors). It is not production code and is not part of
the workspace.

- `probe/src/`: the Rust sources (about 1,400 lines). Build them as a binary crate with toolchain 1.99.1
  (`probe/rust-toolchain.toml.txt`) and these dependencies: `rand_core 0.9`, `rand_pcg 0.9`,
  `png 0.17`, `sha2 0.10`. The `Cargo.toml` is left out so the gate does not treat this folder as
  code (`docs/gate.md`).
- `probe/results/round-1.txt`: the first round (sparse space only).
- `probe/results/round-2.txt`: the second round (sparse and dense, three recipes, seeds 1–8, every
  run repeated for determinism).

The pictures it wrote are not kept (G10); the spec describes what they showed.
