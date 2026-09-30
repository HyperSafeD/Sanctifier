import { describe, it, expect, beforeAll } from "vitest";
import { readFileSync } from "fs";
import { resolve } from "path";

// `globals.css` has no exported JS surface to import, so these are
// structural/content tests over the raw stylesheet text rather than a
// rendered-DOM test — the practical way to "unit test" a plain CSS file
// without a real browser layout/paint engine (happy-dom's CSS support is too
// limited to reliably compute cascaded custom-property values).
let css: string;

beforeAll(() => {
  css = readFileSync(resolve(__dirname, "globals.css"), "utf-8");
});

describe("globals.css", () => {
  it("is non-empty and imports Tailwind", () => {
    expect(css.length).toBeGreaterThan(0);
    expect(css).toContain('@import "tailwindcss"');
  });

  it("has balanced braces (basic syntax sanity)", () => {
    const opens = (css.match(/\{/g) ?? []).length;
    const closes = (css.match(/\}/g) ?? []).length;
    expect(opens).toBe(closes);
    expect(opens).toBeGreaterThan(0);
  });

  it("wires the `dark:` Tailwind variant to the `.dark` class", () => {
    // Regression guard for the exact bug the leading comment warns about:
    // without this, `dark:` utilities silently stop responding to the theme
    // toggle and fall back to the OS-level prefers-color-scheme media query.
    expect(css).toMatch(/@custom-variant\s+dark\s*\(&:where\(\.dark,\s*\.dark \*\)\)/);
  });

  describe("light theme (:root defaults)", () => {
    it("defines --background and --foreground", () => {
      const rootBlock = css.match(/:root\s*\{([^}]*)\}/)?.[1] ?? "";
      expect(rootBlock).toMatch(/--background:\s*#[0-9a-fA-F]{6}/);
      expect(rootBlock).toMatch(/--foreground:\s*#[0-9a-fA-F]{6}/);
    });

    it("maps --color-background/--color-foreground onto them for Tailwind's @theme", () => {
      const themeBlock = css.match(/@theme inline\s*\{([^}]*)\}/)?.[1] ?? "";
      expect(themeBlock).toContain("--color-background: var(--background)");
      expect(themeBlock).toContain("--color-foreground: var(--foreground)");
    });
  });

  describe("dark theme override", () => {
    it("targets both the [data-theme=dark] attribute and the .dark class", () => {
      // Edge case: the app supports two different ways of flagging dark mode
      // (an explicit data-theme attribute and Tailwind's .dark class) — both
      // selectors must be present or one of the two toggle mechanisms silently
      // does nothing.
      expect(css).toMatch(/:root\[data-theme="dark"\],\s*\n?\s*:root\.dark\s*\{/);
    });

    it("overrides both --background and --foreground to different values than light mode", () => {
      const rootBlock = css.match(/:root\s*\{([^}]*)\}/)?.[1] ?? "";
      const lightBg = rootBlock.match(/--background:\s*(#[0-9a-fA-F]{6})/)?.[1];
      const lightFg = rootBlock.match(/--foreground:\s*(#[0-9a-fA-F]{6})/)?.[1];

      const darkBlock = css.match(
        /:root\[data-theme="dark"\],\s*\n?\s*:root\.dark\s*\{([^}]*)\}/,
      )?.[1];
      expect(darkBlock).toBeDefined();
      const darkBg = darkBlock?.match(/--background:\s*(#[0-9a-fA-F]{6})/)?.[1];
      const darkFg = darkBlock?.match(/--foreground:\s*(#[0-9a-fA-F]{6})/)?.[1];

      expect(darkBg).toBeDefined();
      expect(darkFg).toBeDefined();
      expect(darkBg).not.toBe(lightBg);
      expect(darkFg).not.toBe(lightFg);
    });

    it("sets color-scheme: dark on <body> so native form controls also switch", () => {
      expect(css).toMatch(
        /:root\[data-theme="dark"\] body,\s*\n?\s*:root\.dark body\s*\{\s*color-scheme:\s*dark;?\s*\}/,
      );
    });
  });

  describe("high-contrast accessibility mode", () => {
    it("defines every custom property the base theme relies on, plus primary/border/ring", () => {
      const block = css.match(/\.theme-high-contrast\s*\{([^}]*)\}/)?.[1] ?? "";
      const required = [
        "--background",
        "--foreground",
        "--primary",
        "--primary-foreground",
        "--muted",
        "--muted-foreground",
        "--border",
        "--input",
        "--ring",
      ];
      for (const prop of required) {
        expect(block, `expected ${prop} in .theme-high-contrast`).toContain(prop);
      }
    });

    it("uses pure black/white/yellow for maximum contrast, not a muted palette", () => {
      const block = css.match(/\.theme-high-contrast\s*\{([^}]*)\}/)?.[1] ?? "";
      expect(block).toMatch(/--background:\s*#000000/);
      expect(block).toMatch(/--foreground:\s*#ffffff/);
      expect(block).toMatch(/--primary:\s*#ffff00/);
    });
  });

  describe("animation keyframes referenced by utility classes", () => {
    // Edge case this guards against: a class referencing a keyframe name that
    // was renamed or removed elsewhere in the file — CSS doesn't error on a
    // missing @keyframes, it just silently skips the animation.
    const referencedAnimations: Array<[className: string, keyframeName: string]> = [
      ["animate-in", "fade-in"],
      ["slide-in-from-bottom-3", "slide-in-from-bottom"],
    ];

    it.each(referencedAnimations)(
      "%s references an animation whose @keyframes %s is defined",
      (className, keyframeName) => {
        const classBlock = css.match(
          new RegExp(`\\.${className}\\s*\\{([^}]*)\\}`),
        )?.[1];
        expect(classBlock).toBeDefined();
        expect(classBlock).toContain(keyframeName);
        expect(css).toMatch(new RegExp(`@keyframes\\s+${keyframeName}\\s*\\{`));
      },
    );

    it("every @keyframes block has both a starting and ending state", () => {
      const keyframeBlocks = [...css.matchAll(/@keyframes\s+[\w-]+\s*\{([^{}]*(?:\{[^{}]*\}[^{}]*)*)\}/g)];
      expect(keyframeBlocks.length).toBeGreaterThan(0);
      for (const [, body] of keyframeBlocks) {
        expect(body).toMatch(/from|0%/);
        expect(body).toMatch(/to|100%/);
      }
    });
  });
});
