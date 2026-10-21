/**
 * BOS Combobox / Table Relation Lookup Types
 * Enterprise multi-column lookup component inspired by Microsoft Dynamics 365 Business Central / NAV.
 */

/**
 * @typedef {'sm' | 'md' | 'lg'} ComboboxDensity
 * @typedef {'outline' | 'filled' | 'underline' | 'ghost'} ComboboxVariant
 * @typedef {'none' | 'sm' | 'md' | 'lg' | 'pill'} ComboboxRadius
 * @typedef {'default' | 'error' | 'warning' | 'success'} ComboboxStatus
 *
 * @typedef {Object} ComboboxColumn
 * @property {string} key - Property path in the row object (e.g. 'code', 'name')
 * @property {string} label - Column header title (e.g. 'No.', 'Customer Name')
 * @property {string} [width] - Explicit CSS column width (e.g. '120px', '25%')
 * @property {string} [minWidth] - Minimum CSS column width
 * @property {'left' | 'center' | 'right'} [align='left'] - Text alignment
 * @property {boolean} [mono=false] - Format with monospace typography (for ERP codes)
 * @property {boolean} [primary=false] - Primary display column
 * @property {'text' | 'currency' | 'number' | 'badge'} [format='text'] - Value formatter
 * @property {boolean} [sortable=true] - Allow sorting by this column
 *
 * @typedef {Object} ComboboxProps
 * @property {any | any[]} [value] - Two-way bindable selected value (single primitive or array if multiple=true)
 * @property {any[]} [items=[]] - Available records to pick from
 * @property {string} [valueKey='id'] - Key in item object to use as value
 * @property {string} [labelKey='label'] - Key in item object to use as display label
 * @property {ComboboxColumn[]} [columns=[]] - Column configuration for multi-column dropdown
 * @property {boolean} [multiple=false] - Multi-select mode with removable chip tokens
 * @property {boolean} [sortable=true] - Allows clicking column headers to sort ascending/descending
 * @property {string} [groupBy] - Property path to group records by (e.g. 'category', 'city')
 * @property {string} [placeholder='Select an option...'] - Watermark text
 * @property {string} [searchPlaceholder='Search records...'] - Dropdown search placeholder
 * @property {string} [label] - Top form field label
 * @property {string} [helperText] - Subordinate explanatory text
 * @property {string} [errorMessage] - Inline error feedback message
 * @property {string} [warningMessage] - Inline warning feedback message
 * @property {string} [currency='USD'] - ISO 4217 currency code for currency-formatted columns
 * @property {string} [locale='en-US'] - BCP 47 locale tag for number and currency formatting
 * @property {string[] | null} [factBoxFields=null] - Explicit list of record keys to render in FactBox preview footer
 * @property {ComboboxDensity} [density='md'] - Sizing density ('sm' compact, 'md' standard, 'lg' spacious)
 * @property {ComboboxVariant} [variant='outline'] - Border & surface style ('outline', 'filled', 'underline', 'ghost')
 * @property {ComboboxRadius} [rounded='md'] - Corner roundness ('none' sharp, 'sm', 'md', 'pill')
 * @property {ComboboxStatus} [status='default'] - Validation state
 * @property {boolean} [required=false] - Mandates input
 * @property {boolean} [disabled=false] - Disables interactions
 * @property {boolean} [readonly=false] - Makes field immutable
 * @property {boolean} [clearable=true] - Allows clearing selection via (X) button
 * @property {boolean} [loading=false] - Shows async loading spinner
 * @property {boolean} [showFactBox=true] - Renders hover preview card with extra metadata
 * @property {string} [newRecordLabel] - Label for quick-create button (e.g. "+ New Customer")
 * @property {string} [fullListLabel='Select from full list (F4)'] - Label for full-page lookup
 * @property {string} [persistRecentKey] - LocalStorage key to persist and display recently chosen records
 * @property {number} [maxDisplayItems=100] - Cap on rendered rows for high-volume performance
 * @property {(query: string) => Promise<any[]>} [fetchItems] - Async remote API data fetcher
 * @property {() => void} [onCreateNew] - Callback when "+ New" action is triggered
 * @property {() => void} [onFullList] - Callback when "Select from full list" is triggered
 * @property {(query: string) => void} [onSearch] - Search callback for server-side lookups
 * @property {(item: any) => void} [onSelect] - Callback when an item is selected
 * @property {(item: any) => void} [onRemove] - Callback when an item token is removed in multi-select mode
 * @property {() => void} [onClear] - Callback when selection is cleared
 */

export const ComboboxVariants = {
  OUTLINE: 'outline',
  FILLED: 'filled',
  UNDERLINE: 'underline',
  GHOST: 'ghost'
};

export const ComboboxDensities = {
  SM: 'sm', // Compact 28px
  MD: 'md', // Standard 36px
  LG: 'lg'  // Spacious 44px
};
