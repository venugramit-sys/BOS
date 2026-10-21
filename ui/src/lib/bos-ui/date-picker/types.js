/**
 * BOS Enterprise Date, Date Range & Time Picker Types
 * Inspired by Microsoft Dynamics 365 Business Central / NAV.
 */

/**
 * @typedef {'sm' | 'md' | 'lg'} DatePickerDensity
 * @typedef {'outline' | 'filled' | 'underline' | 'ghost'} DatePickerVariant
 * @typedef {'none' | 'sm' | 'md' | 'lg' | 'pill'} DatePickerRadius
 * @typedef {'default' | 'error' | 'warning' | 'success'} DatePickerStatus
 *
 * @typedef {Object} DateRangeValue
 * @property {string} [start] - ISO start date (YYYY-MM-DD)
 * @property {string} [end] - ISO end date (YYYY-MM-DD)
 *
 * @typedef {Object} ClosedPeriod
 * @property {string} start - ISO start date of closed period
 * @property {string} end - ISO end date of closed period
 * @property {string} name - e.g. "FY2025 Period 12 - Audited"
 *
 * @typedef {Object} DatePickerProps
 * @property {string | DateRangeValue} [value] - Two-way bindable date string (YYYY-MM-DD) or range object
 * @property {boolean} [range=false] - Enables dual-date range picking mode
 * @property {boolean} [showTime=false] - Enables embedded time selection
 * @property {string} [timeValue='12:00'] - Two-way bindable time string (HH:mm)
 * @property {number} [timeStep=15] - Snapping interval for minutes (e.g. 15 or 30)
 * @property {boolean} [evalFormula=true] - Enables Business Central formula evaluation ('t', 'cm+10d', '+30d')
 * @property {boolean} [allowClosingDates=false] - Enables 'C' prefix closing dates for year-end adjustments
 * @property {ClosedPeriod[]} [closedPeriods=[]] - Prohibited / locked accounting periods
 * @property {string} [label] - Top form field label
 * @property {string} [helperText] - Subordinate explanatory text
 * @property {string} [errorMessage] - Inline error feedback message
 * @property {string} [warningMessage] - Inline warning feedback message
 * @property {string} [placeholder='YYYY-MM-DD or formula (e.g. cm+10d)'] - Watermark text
 * @property {DatePickerDensity} [density='md'] - Sizing density
 * @property {DatePickerVariant} [variant='outline'] - Border & surface style
 * @property {DatePickerRadius} [rounded='md'] - Corner radius style
 * @property {DatePickerStatus} [status='default'] - Validation state
 * @property {boolean} [required=false] - Mandates input
 * @property {boolean} [disabled=false] - Disables interactions
 * @property {boolean} [readonly=false] - Makes input immutable
 * @property {boolean} [clearable=true] - Allows 1-click clearing
 * @property {Date} [minDate] - Minimum permissible date
 * @property {Date} [maxDate] - Maximum permissible date
 * @property {(val: string | DateRangeValue) => void} [onChange] - Value change callback
 * @property {(calcResult: any) => void} [onFormulaCalculated] - Fired when formula is evaluated
 * @property {() => void} [onClear] - Clear callback
 */

export const DatePickerVariants = {
  OUTLINE: 'outline',
  FILLED: 'filled',
  UNDERLINE: 'underline',
  GHOST: 'ghost'
};

export const DatePickerDensities = {
  SM: 'sm', // Compact 28px
  MD: 'md', // Standard 36px
  LG: 'lg'  // Spacious 44px
};
