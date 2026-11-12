# Decisions at checkpoint 1 (Jann, 11 Nov 2026)

These replace the open questions of `02-band-requirements.md` and `03-floor-frame.md` as they
stood at the checkpoint.

## Band requirements

- The eight terrain types (A–H) and the scales per band are about right.
- **Band 3:** the kilometre-tall trees are climbed. They are terrain, not scenery (new terrain
  type **I · Giant trees**). Details are not to be fixed now.
- **Band 4:** both natural cavern and built labyrinth. The early floors are natural caverns and
  the band turns into labyrinth as it rises.
- **Bands blend.** The environment changes gradually towards the next band and from the band
  below; there are no hard cuts at a band's edge. Every later stage checks whether the generator
  can blend two terrain types on one floor and shift a recipe step by step over ten floors.

## Floor frame

- **The floor's physics are not to be taken strictly. It is magic.** The vault's height does not
  limit relief. Weather, temperature and light need no physical reason under a vault. Questions 3,
  4 and 5 of `03-floor-frame.md` are dropped as blockers; light as a factor for vegetation stays a
  cheap option where a band needs it (the Underdeep).
- Not answered by Jann, so the recommendations stand as working assumptions:
  - the Tower proposal's floor model (vault, light-strip, rim over the Bore, free size);
  - a map shows a region of a floor, a whole floor only where the floor is small;
  - every floor has Chimes, a gate up with its guardian, a gate down, and a hub settlement;
  - the Hem (floor 0) is one test case for the city topic.

## Delivery

- The research is saved to the editor repository in a pull request at the end of the workflow,
  as Markdown and, like the prose editor research, with the probe code.
