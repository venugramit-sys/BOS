/**
 * @typedef {Object} MenuItemBase
 * @property {string} [id]
 * @property {'item' | 'separator' | 'submenu' | 'checkbox' | 'radio-group' | 'group' | 'custom'} [type]
 * @property {string} [label]
 * @property {any} [icon] - Lucide icon component or Svelte component
 * @property {string} [shortcut] - e.g. "⌘S" or "Ctrl+P"
 * @property {string} [badge] - Optional badge text
 * @property {'default' | 'accent' | 'warning' | 'destructive'} [badgeVariant]
 * @property {boolean} [disabled]
 * @property {boolean} [destructive]
 * @property {string} [href] - Navigation URL
 * @property {string} [target] - Navigation target (e.g. "_blank")
 * @property {() => void | Promise<void>} [action] - Click callback handler
 * @property {boolean} [checked] - For 'checkbox' type
 * @property {(checked: boolean) => void} [onCheckedChange] - For 'checkbox' type
 * @property {string} [value] - Current value for 'radio-group'
 * @property {(value: string) => void} [onValueChange] - For 'radio-group'
 * @property {{ value: string, label: string, disabled?: boolean }[]} [options] - Options for 'radio-group'
 * @property {string} [heading] - Heading text for 'group' type
 * @property {MenuItem[]} [items] - Children for 'submenu' or 'group'
 * @property {any} [custom] - Custom payload or component for 'custom' type
 */

/**
 * @typedef {MenuItemBase} MenuItem
 */

/**
 * @typedef {Object} MenuTopLevel
 * @property {string} [id]
 * @property {string} label - Top level trigger label
 * @property {any} [icon] - Optional trigger icon
 * @property {boolean} [disabled]
 * @property {string} [triggerClass]
 * @property {string} [contentClass]
 * @property {'start' | 'center' | 'end'} [align]
 * @property {MenuItem[]} items - Child items in dropdown
 */

/**
 * Helper builder functions for expressive type-safe schema composition
 */
export const MenuHelpers = {
  item: (label, action, options = {}) => ({
    type: 'item',
    label,
    action,
    ...options
  }),
  separator: () => ({
    type: 'separator'
  }),
  group: (heading, items) => ({
    type: 'group',
    heading,
    items
  }),
  submenu: (label, items, options = {}) => ({
    type: 'submenu',
    label,
    items,
    ...options
  }),
  checkbox: (label, checked, onCheckedChange, options = {}) => ({
    type: 'checkbox',
    label,
    checked,
    onCheckedChange,
    ...options
  }),
  radioGroup: (value, options, onValueChange) => ({
    type: 'radio-group',
    value,
    options,
    onValueChange
  })
};
