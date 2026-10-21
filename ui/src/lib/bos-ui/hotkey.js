/**
 * Match a keyboard event to a shortcut string such as "Ctrl+K" or "F5".
 * Ctrl also accepts the Command key so the same map works on macOS.
 * @param {KeyboardEvent} event
 * @param {string} shortcut
 */
export function eventMatchesShortcut(event, shortcut) {
  if (!event || !shortcut) return false;
  const parts = shortcut
    .toLowerCase()
    .split("+")
    .map((part) => part.trim())
    .filter(Boolean);
  if (parts.length === 0) return false;

  const key = parts[parts.length - 1];
  const needMod = parts.some((part) => part === "ctrl" || part === "cmd" || part === "mod" || part === "meta");
  const needShift = parts.includes("shift");
  const needAlt = parts.includes("alt");
  const hasMod = event.ctrlKey || event.metaKey;

  if (needMod !== hasMod) return false;
  if (needShift !== event.shiftKey) return false;
  if (needAlt !== event.altKey) return false;

  return event.key.toLowerCase() === key;
}

/**
 * @param {string} shortcut
 * @returns {string[]}
 */
export function shortcutParts(shortcut) {
  if (!shortcut) return [];
  return shortcut.split("+").map((part) => part.trim()).filter(Boolean);
}
