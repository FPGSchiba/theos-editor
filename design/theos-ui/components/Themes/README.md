Grid Noir and Grid Light side by side: the same markup, switched only by `data-theme`.

- Set the theme once on the root: `<html data-theme="grid-light">`. Without the attribute, `:root` is Grid Noir.
- The attribute also works on any element, which is how this card shows both. Use that for previews and docs only; an app has one theme.
- Components provide nothing per theme. If a component needs a theme check, it has a hard-coded colour: replace it with a token.
- The consumer provides the switch (a setting, or `prefers-color-scheme` mapped to the two ids) and stores the choice.
