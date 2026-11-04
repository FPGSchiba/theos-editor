import type { BeatKey, Kind, LinkPlace } from './types.ts';

/** Every kind, in the order the data model lists them. */
export const ALL_KINDS: readonly Kind[] = [
  'tower', 'band', 'floor', 'location', 'people', 'faction', 'npc',
  'thing', 'topic', 'truth', 'belief', 'clue', 'equation', 'script', 'dialogue',
  'template', 'concept', 'concept-version', 'map', 'map-version',
];

/** Data model §9.4: the kinds an attribute line of a beat may link to. */
const BEAT_KINDS: Record<BeatKey, readonly Kind[]> = {
  at: ['location', 'floor'],
  with: ['npc', 'faction', 'people'],
  says: ['belief'],
  reveals: ['clue'],
  when: [],
};

/** The kinds a `[[` link may point at in this place: every kind in prose, §9.4 in a beat. */
export function allowedKinds(place: LinkPlace): Kind[] {
  return place.in === 'prose' ? [...ALL_KINDS] : [...BEAT_KINDS[place.key]];
}
