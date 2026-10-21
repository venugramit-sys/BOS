/**
 * @typedef {'text' | 'password' | 'number' | 'currency' | 'search' | 'lookup' | 'code' | 'email' | 'url' | 'tel' | 'masked'} InputMode
 * @typedef {'sm' | 'md' | 'lg'} InputSize
 * @typedef {'default' | 'error' | 'warning' | 'success' | 'info'} InputStatus
 * @typedef {'credit-card' | 'ssn' | 'iban' | 'custom'} MaskType
 *
 * @typedef {Object} InputProps
 * @property {string | number} [value] - Two-way bindable field value
 * @property {InputMode} [type='text'] - Polymorphic input control type
 * @property {InputSize} [size='md'] - Sizing variant
 * @property {InputStatus} [status='default'] - Validation state
 * @property {string} [label] - Top form field label
 * @property {string} [helperText] - Subordinate explanatory text
 * @property {string} [errorMessage] - Inline error feedback message
 * @property {string} [warningMessage] - Inline warning feedback message
 * @property {string} [successMessage] - Inline success feedback message
 * @property {string} [placeholder] - Watermark text
 * @property {boolean} [required=false] - Mandates input
 * @property {boolean} [disabled=false] - Disables interactions
 * @property {boolean} [readonly=false] - Makes text immutable
 * @property {string} [prefixText] - Leading static addon text (e.g. "$", "https://")
 * @property {any} [prefixIcon] - Leading icon component
 * @property {string} [suffixText] - Trailing static addon text (e.g. "USD", "kg")
 * @property {any} [suffixIcon] - Trailing icon component
 * @property {boolean} [clearable=false] - Shows 1-click clear button when non-empty
 * @property {boolean} [copyable=false] - Shows 1-click clipboard copy button
 * @property {boolean} [loading=false] - Renders asynchronous activity spinner
 * @property {boolean} [stepper=false] - Renders increment/decrement stepper controls for numbers
 * @property {boolean} [autoUppercase=false] - Automatically forces uppercase text (ERP Code convention)
 * @property {boolean} [evalMath=false] - Enables in-field arithmetic calculation on blur/enter (e.g. 50 * 1.2 -> 60)
 * @property {boolean} [masked=false] - Enables sensitive field masking (e.g. •••• 4242)
 * @property {MaskType} [maskType='custom'] - Mask pattern type
 * @property {boolean} [unmaskable=false] - Allows user to toggle revealing raw text via lock/unlock button
 * @property {number} [decimals] - Decimal precision for numbers/currency (default 2)
 * @property {boolean} [formatOnBlur=true] - Formats numbers with thousand separators on blur
 * @property {number} [min] - Minimum numeric threshold
 * @property {number} [max] - Maximum numeric threshold
 * @property {number} [step=1] - Stepper increment amount
 * @property {number} [maxlength] - Maximum character boundary
 * @property {boolean} [showCount=false] - Displays live character counter
 * @property {number} [debounceMs=300] - Milliseconds to delay debounced event
 * @property {(val: string | number) => void} [onDebounce] - Debounced value change callback
 * @property {() => void} [onClear] - Clear action callback
 * @property {() => void} [onLookup] - AssistEdit / Relation lookup trigger callback
 * @property {(val: string) => void} [onCopy] - Copy to clipboard callback
 * @property {(res: number, originalExpr: string) => void} [onCalculate] - Fired when an arithmetic formula is evaluated
 */

export const InputTypes = {
  TEXT: 'text',
  PASSWORD: 'password',
  NUMBER: 'number',
  CURRENCY: 'currency',
  SEARCH: 'search',
  LOOKUP: 'lookup',
  CODE: 'code',
  EMAIL: 'email',
  URL: 'url',
  TEL: 'tel',
  MASKED: 'masked'
};
