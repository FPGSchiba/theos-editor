// The rules of the style test (P11 §5.7). Svelte's own parser reads the component and its CSS;
// each rule looks at one declaration. A new rule is one entry in `RULES`.

import colorNames from 'color-name';
import { parse } from 'svelte/compiler';

/** One CSS declaration and where it was written (`<style>`, `style` or `style:<name>`). */
interface Declaration {
  where: string;
  property: string;
  value: string;
}

/** A rule: the breach it finds in a declaration, or `undefined`. */
type Rule = (declaration: Declaration) => string | undefined;

const NAMED_COLOURS = new Set(Object.keys(colorNames));
const HEX = /#[0-9a-f]{3,8}\b/i;
const COLOUR_FUNCTION = /\b(?:rgba?|hsla?|hwb|lab|lch|oklab|oklch|color)\(/i;
const WORD = /(?<![\w-])[a-z]+(?![\w(-])/gi;
const TOKEN = /^var\(--[\w-]+\)$/;
const PX = /\d(?:\.\d+)?px\b/;

const shown = (d: Declaration): string => `\`${d.property}: ${d.value}\``;

const colourNamed = (d: Declaration): string | undefined => {
  const named = d.value.match(WORD)?.find((word) => NAMED_COLOURS.has(word.toLowerCase()));
  const literal = HEX.exec(d.value)?.[0] ?? COLOUR_FUNCTION.exec(d.value)?.[0] ?? named;
  return literal === undefined
    ? undefined
    : `${shown(d)} names a colour (\`${literal}\`); use a var(--…) token`;
};

const fontFamily = (d: Declaration): string | undefined =>
  (d.property === 'font-family' || d.property === 'font') && !TOKEN.test(d.value.trim())
    ? `${shown(d)} is not a var(--…) token`
    : undefined;

const opacity = (d: Declaration): string | undefined =>
  d.property === 'opacity' ? `${shown(d)} fades text; use a dimmer colour token` : undefined;

const pxFontSize = (d: Declaration): string | undefined =>
  (d.property === 'font-size' || d.property === 'font') && PX.test(d.value)
    ? `${shown(d)} is a px font size; use a token or a tu-* class`
    : undefined;

const RULES: readonly Rule[] = [colourNamed, fontFamily, opacity, pxFontSize];

/** Every node of `type` in a Svelte AST, in source order. */
function nodesOf(tree: unknown, type: string): Record<string, unknown>[] {
  const found: Record<string, unknown>[] = [];
  const visit = (node: unknown): void => {
    if (Array.isArray(node)) {
      node.forEach(visit);
    } else if (node !== null && typeof node === 'object') {
      const record = node as Record<string, unknown>;
      if (record['type'] === type) {
        found.push(record);
      }
      Object.values(record).forEach(visit);
    }
  };
  visit(tree);
  return found;
}

/** The written text of an attribute value; expressions (`{…}`) are left out. */
function staticText(value: unknown): string {
  return nodesOf(value, 'Text')
    .map((text) => String(text['data']))
    .join('');
}

/** The declarations of the CSS `text`, read by Svelte's parser. */
function declarationsOf(css: unknown, where: string): Declaration[] {
  return nodesOf(css, 'Declaration').map((d) => ({
    where,
    property: String(d['property']).toLowerCase(),
    value: String(d['value']),
  }));
}

/** The declarations of an inline `style="…"`, parsed as the body of a rule. */
function inlineDeclarations(text: string, where: string): Declaration[] {
  return declarationsOf(parse(`<style>x{${text}}</style>`, { modern: true }).css, where);
}

/** Every breach of the rules in the component `source`, as `<where>: <breach>`. */
export function findStyleViolations(source: string): string[] {
  const root = parse(source, { modern: true });
  const declarations = [
    ...nodesOf(root.fragment, 'Attribute')
      .filter((a) => a['name'] === 'style')
      .flatMap((a) => inlineDeclarations(staticText(a['value']), 'style')),
    ...nodesOf(root.fragment, 'StyleDirective').map((s) => ({
      where: `style:${String(s['name'])}`,
      property: String(s['name']).toLowerCase(),
      value: staticText(s['value']),
    })),
    ...declarationsOf(root.css, '<style>'),
  ];
  return declarations.flatMap((d) =>
    RULES.flatMap((rule) => {
      const breach = rule(d);
      return breach === undefined ? [] : [`${d.where}: ${breach}`];
    }),
  );
}
