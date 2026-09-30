/**
 * Global keyboard shortcuts.
 *
 * `resolveShortcut` is a pure key → intent mapping so it can be unit tested.
 * Page-specific intents (save, upload, run) are broadcast as a window event,
 * letting the page that owns the report/file input react to them.
 */

export const SHORTCUT_EVENT = "sanctifier:shortcut";

export type ShortcutAction = "save" | "upload" | "run";

/** Routes reachable with ⌘/Ctrl + 1…5, in order. */
export const TAB_ROUTES: Array<{ label: string; href: string }> = [
  { label: "Scan", href: "/scan" },
  { label: "Dashboard", href: "/dashboard" },
  { label: "Contracts", href: "/contracts" },
  { label: "Audit", href: "/audit" },
  { label: "Issues", href: "/issues" },
];

export const SHORTCUT_HELP: Array<{ keys: string; description: string }> = [
  { keys: "⌘/Ctrl + S", description: "Save or export the current report" },
  { keys: "⌘/Ctrl + U", description: "Upload a contract file" },
  { keys: "⌘/Ctrl + ↵", description: "Run analysis" },
  { keys: "⌘/Ctrl + 1…5", description: `Go to ${TAB_ROUTES.map((t) => t.label).join(", ")}` },
  { keys: "⌘/Ctrl + K", description: "Open the command palette" },
  { keys: "?", description: "Show this shortcuts help" },
];

export type ShortcutResolution =
  | { kind: "action"; action: ShortcutAction }
  | { kind: "navigate"; href: string }
  | { kind: "help" };

export interface ShortcutKeyEvent {
  key: string;
  metaKey?: boolean;
  ctrlKey?: boolean;
  altKey?: boolean;
  /** True when the event came from a text field, where `?` must stay a character. */
  isEditableTarget?: boolean;
}

export function resolveShortcut(event: ShortcutKeyEvent): ShortcutResolution | null {
  const mod = Boolean(event.metaKey || event.ctrlKey);

  if (mod && !event.altKey) {
    const key = event.key.toLowerCase();
    if (key === "s") return { kind: "action", action: "save" };
    if (key === "u") return { kind: "action", action: "upload" };
    if (key === "enter") return { kind: "action", action: "run" };

    const tab = Number(event.key);
    if (Number.isInteger(tab) && tab >= 1 && tab <= TAB_ROUTES.length) {
      return { kind: "navigate", href: TAB_ROUTES[tab - 1].href };
    }
    return null;
  }

  if (event.key === "?" && !event.isEditableTarget) return { kind: "help" };
  return null;
}

export function isEditableTarget(target: EventTarget | null): boolean {
  const element = target as HTMLElement | null;
  if (!element?.tagName) return false;
  const tag = element.tagName.toLowerCase();
  return tag === "input" || tag === "textarea" || tag === "select" || element.isContentEditable === true;
}

export function emitShortcut(action: ShortcutAction): void {
  if (typeof window === "undefined") return;
  window.dispatchEvent(new CustomEvent<ShortcutAction>(SHORTCUT_EVENT, { detail: action }));
}

/** Subscribe to one shortcut action; returns an unsubscribe function for `useEffect`. */
export function onShortcut(action: ShortcutAction, handler: () => void): () => void {
  if (typeof window === "undefined") return () => {};
  const listener = (event: Event) => {
    if ((event as CustomEvent<ShortcutAction>).detail === action) handler();
  };
  window.addEventListener(SHORTCUT_EVENT, listener);
  return () => window.removeEventListener(SHORTCUT_EVENT, listener);
}
