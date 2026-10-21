//! Page & page extension IR.
//!
//! Two layers live here:
//! - **Authored IR** (`PageIR`, `PageExtensionIR`, `PageControlIR`, `PageActionIR`): what the analyzer lowers
//!   from `.bos` source and what is persisted in `_bos_meta_objects.definition`. Old (v1) JSON still loads
//!   because every v2 addition is `#[serde(default)]`.
//! - **Compiled IR** (`CompiledPageIR`, `ResolvedControlIR`, ...): the fully merged page with every control
//!   resolved against its source table (captions, control types, options, lookups, formats filled in
//!   NAV-style). This is the only input of the Svelte page code generator.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

use crate::{TransitionIR, TriggerIR};

/// Current page IR schema version written by the analyzer.
pub const PAGE_IR_VERSION: u32 = 2;

fn legacy_ir_version() -> u32 {
    1
}

fn default_true() -> bool {
    true
}

fn is_false(b: &bool) -> bool {
    !*b
}

fn normalize_key(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_lowercase()
}

// ----------------------------------------------------
// PAGE TYPES & CONTROL KINDS
// ----------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PageType {
    Card,
    List,
    Document,
    ListPart,
    CardPart,
    Worksheet,
    StandardDialog,
    ConfirmationDialog,
    NavigatePage,
    RoleCenter,
    Api,
    Board,
    Timeline,
}

impl PageType {
    pub const ALL: [PageType; 13] = [
        PageType::Card,
        PageType::List,
        PageType::Document,
        PageType::ListPart,
        PageType::CardPart,
        PageType::Worksheet,
        PageType::StandardDialog,
        PageType::ConfirmationDialog,
        PageType::NavigatePage,
        PageType::RoleCenter,
        PageType::Api,
        PageType::Board,
        PageType::Timeline,
    ];

    /// Parses an authored page type, accepting the legacy designer layout names as aliases.
    pub fn parse(raw: &str) -> Option<PageType> {
        Some(match normalize_key(raw).as_str() {
            "card" | "record" | "form" => PageType::Card,
            "list" => PageType::List,
            "document" => PageType::Document,
            "listpart" | "subform" | "lines" => PageType::ListPart,
            "cardpart" | "factbox" => PageType::CardPart,
            "worksheet" | "journal" | "grid" => PageType::Worksheet,
            "standarddialog" | "dialog" => PageType::StandardDialog,
            "confirmationdialog" | "confirm" => PageType::ConfirmationDialog,
            "navigatepage" | "wizard" | "assisted" => PageType::NavigatePage,
            "rolecenter" | "workspace" | "home" | "dashboard" => PageType::RoleCenter,
            "api" => PageType::Api,
            "board" | "kanban" => PageType::Board,
            "timeline" => PageType::Timeline,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            PageType::Card => "Card",
            PageType::List => "List",
            PageType::Document => "Document",
            PageType::ListPart => "ListPart",
            PageType::CardPart => "CardPart",
            PageType::Worksheet => "Worksheet",
            PageType::StandardDialog => "StandardDialog",
            PageType::ConfirmationDialog => "ConfirmationDialog",
            PageType::NavigatePage => "NavigatePage",
            PageType::RoleCenter => "RoleCenter",
            PageType::Api => "Api",
            PageType::Board => "Board",
            PageType::Timeline => "Timeline",
        }
    }

    /// Pages that show many records of their source table at once.
    pub fn is_list_like(self) -> bool {
        matches!(self, PageType::List | PageType::ListPart | PageType::Worksheet | PageType::Board | PageType::Timeline)
    }

    /// Pages that are only embedded inside another page through a `part`.
    pub fn is_part(self) -> bool {
        matches!(self, PageType::ListPart | PageType::CardPart)
    }

    pub fn is_dialog(self) -> bool {
        matches!(self, PageType::StandardDialog | PageType::ConfirmationDialog)
    }

    /// `Api` pages publish data only and never produce a UI bundle.
    pub fn has_ui(self) -> bool {
        !matches!(self, PageType::Api)
    }

    /// Whether a control kind may appear on this page type (directives are always allowed in extensions).
    pub fn allows(self, kind: PageControlKind) -> bool {
        use PageControlKind as K;
        match kind {
            K::Other(_) => false,
            k if k.is_directive() => true,
            K::Repeater => self.is_list_like() || matches!(self, PageType::Document | PageType::NavigatePage | PageType::Api),
            K::CueGroup | K::Cue => matches!(self, PageType::RoleCenter | PageType::CardPart),
            K::WizardStep => matches!(self, PageType::NavigatePage),
            K::Part | K::SystemPart => !self.is_part() || matches!(self, PageType::CardPart),
            K::Headline => matches!(self, PageType::RoleCenter),
            _ => true,
        }
    }
}

impl fmt::Display for PageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Canonical control kinds. The authored IR keeps `kind` as a string (stable JSON); this enum is the
/// typed view used by the resolver, the code generator and the designer property schema.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PageControlKind {
    Area,
    Group,
    Grid,
    Fixed,
    Repeater,
    Part,
    SystemPart,
    Field,
    Label,
    Image,
    Spacer,
    Divider,
    CueGroup,
    Cue,
    Headline,
    UserControl,
    Tabs,
    Tab,
    Flex,
    Split,
    Card,
    WizardStep,
    Container,
    AddFirst,
    AddLast,
    AddBefore,
    AddAfter,
    Modify,
    MoveFirst,
    MoveLast,
    MoveBefore,
    MoveAfter,
    Replace,
    Hide,
    Wrap,
    Other(String),
}

impl PageControlKind {
    pub fn parse(raw: &str) -> PageControlKind {
        use PageControlKind as K;
        match normalize_key(raw).as_str() {
            "area" | "region" | "regions" => K::Area,
            "group" | "section" | "fasttab" | "accordion" | "panel" | "header" | "footer" | "main" | "side" | "content" => K::Group,
            "grid" | "columns" | "column" | "bento" => K::Grid,
            "fixed" => K::Fixed,
            "repeater" | "lines" | "kanban" => K::Repeater,
            "part" | "factbox" | "subform" => K::Part,
            "systempart" => K::SystemPart,
            "field" => K::Field,
            "label" | "text" => K::Label,
            "image" => K::Image,
            "spacer" => K::Spacer,
            "divider" | "separator" => K::Divider,
            "cuegroup" | "tiles" => K::CueGroup,
            "cue" | "tile" => K::Cue,
            "headline" => K::Headline,
            "usercontrol" | "chart" => K::UserControl,
            "tabs" => K::Tabs,
            "tab" => K::Tab,
            "flex" | "stack" => K::Flex,
            "split" => K::Split,
            "card" | "drawer" | "modal" => K::Card,
            "wizardstep" | "step" | "steps" | "stepper" => K::WizardStep,
            "container" => K::Container,
            "addfirst" => K::AddFirst,
            "addlast" => K::AddLast,
            "addbefore" => K::AddBefore,
            "addafter" => K::AddAfter,
            "modify" => K::Modify,
            "movefirst" => K::MoveFirst,
            "movelast" => K::MoveLast,
            "movebefore" => K::MoveBefore,
            "moveafter" => K::MoveAfter,
            "replace" => K::Replace,
            "hide" => K::Hide,
            "wrap" => K::Wrap,
            _ => K::Other(raw.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        use PageControlKind as K;
        match self {
            K::Area => "area",
            K::Group => "group",
            K::Grid => "grid",
            K::Fixed => "fixed",
            K::Repeater => "repeater",
            K::Part => "part",
            K::SystemPart => "systempart",
            K::Field => "field",
            K::Label => "label",
            K::Image => "image",
            K::Spacer => "spacer",
            K::Divider => "divider",
            K::CueGroup => "cuegroup",
            K::Cue => "cue",
            K::Headline => "headline",
            K::UserControl => "usercontrol",
            K::Tabs => "tabs",
            K::Tab => "tab",
            K::Flex => "flex",
            K::Split => "split",
            K::Card => "card",
            K::WizardStep => "wizardstep",
            K::Container => "container",
            K::AddFirst => "addFirst",
            K::AddLast => "addLast",
            K::AddBefore => "addBefore",
            K::AddAfter => "addAfter",
            K::Modify => "modify",
            K::MoveFirst => "moveFirst",
            K::MoveLast => "moveLast",
            K::MoveBefore => "moveBefore",
            K::MoveAfter => "moveAfter",
            K::Replace => "replace",
            K::Hide => "hide",
            K::Wrap => "wrap",
            K::Other(s) => s.as_str(),
        }
    }

    /// Extension directives (anchor-based edits of the base page tree).
    pub fn is_directive(&self) -> bool {
        use PageControlKind as K;
        matches!(
            self,
            K::AddFirst
                | K::AddLast
                | K::AddBefore
                | K::AddAfter
                | K::Modify
                | K::MoveFirst
                | K::MoveLast
                | K::MoveBefore
                | K::MoveAfter
                | K::Replace
                | K::Hide
                | K::Wrap
        )
    }

    /// Kinds that hold child controls.
    pub fn is_container(&self) -> bool {
        use PageControlKind as K;
        matches!(
            self,
            K::Area | K::Group | K::Grid | K::Fixed | K::Repeater | K::CueGroup | K::Tabs | K::Tab | K::Flex | K::Split | K::Card | K::WizardStep | K::Container
        )
    }

    /// Kinds that bind to a source expression (table column, FlowField or page variable).
    pub fn is_bound(&self) -> bool {
        matches!(self, PageControlKind::Field | PageControlKind::Cue)
    }
}

impl fmt::Display for PageControlKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ----------------------------------------------------
// STYLE & RESPONSIVE LAYOUT
// ----------------------------------------------------

/// Named theme colours. Rendered as CSS custom properties `--bos-color-<token>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorToken {
    Surface,
    SurfaceMuted,
    Primary,
    Secondary,
    Accent,
    Neutral,
    Info,
    Favorable,
    Unfavorable,
    Attention,
    Ambiguous,
    Subordinate,
    Strong,
}

impl ColorToken {
    pub const ALL: [ColorToken; 13] = [
        ColorToken::Surface,
        ColorToken::SurfaceMuted,
        ColorToken::Primary,
        ColorToken::Secondary,
        ColorToken::Accent,
        ColorToken::Neutral,
        ColorToken::Info,
        ColorToken::Favorable,
        ColorToken::Unfavorable,
        ColorToken::Attention,
        ColorToken::Ambiguous,
        ColorToken::Subordinate,
        ColorToken::Strong,
    ];

    pub fn parse(raw: &str) -> Option<ColorToken> {
        Some(match normalize_key(raw).as_str() {
            "surface" => ColorToken::Surface,
            "surfacemuted" | "muted" => ColorToken::SurfaceMuted,
            "primary" => ColorToken::Primary,
            "secondary" => ColorToken::Secondary,
            "accent" => ColorToken::Accent,
            "neutral" => ColorToken::Neutral,
            "info" => ColorToken::Info,
            "favorable" | "success" => ColorToken::Favorable,
            "unfavorable" | "danger" | "error" => ColorToken::Unfavorable,
            "attention" | "warning" => ColorToken::Attention,
            "ambiguous" => ColorToken::Ambiguous,
            "subordinate" => ColorToken::Subordinate,
            "strong" => ColorToken::Strong,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ColorToken::Surface => "surface",
            ColorToken::SurfaceMuted => "surfaceMuted",
            ColorToken::Primary => "primary",
            ColorToken::Secondary => "secondary",
            ColorToken::Accent => "accent",
            ColorToken::Neutral => "neutral",
            ColorToken::Info => "info",
            ColorToken::Favorable => "favorable",
            ColorToken::Unfavorable => "unfavorable",
            ColorToken::Attention => "attention",
            ColorToken::Ambiguous => "ambiguous",
            ColorToken::Subordinate => "subordinate",
            ColorToken::Strong => "strong",
        }
    }
}

/// A colour is either a theme token or an exact `#rrggbb` hex value. Serialized as a plain string
/// (`"primary"` or `"#1e40af"`) so the designer and the JSON catalog stay readable.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum ColorValue {
    Token(ColorToken),
    Hex(String),
}

impl ColorValue {
    /// Parses a token name or a `#rgb` / `#rrggbb` hex colour (normalized to lowercase `#rrggbb`).
    pub fn parse(raw: &str) -> Result<ColorValue, String> {
        let s = raw.trim();
        if let Some(hex) = s.strip_prefix('#') {
            if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(format!("'{s}' is not a valid hex colour"));
            }
            let full = match hex.len() {
                3 => hex.chars().flat_map(|c| [c, c]).collect::<String>(),
                6 => hex.to_string(),
                _ => return Err(format!("'{s}' must be #rgb or #rrggbb")),
            };
            return Ok(ColorValue::Hex(format!("#{}", full.to_ascii_lowercase())));
        }
        ColorToken::parse(s).map(ColorValue::Token).ok_or_else(|| {
            let names: Vec<&str> = ColorToken::ALL.iter().map(|t| t.as_str()).collect();
            format!("'{s}' is not a theme colour; use one of {} or a #rrggbb hex value", names.join(", "))
        })
    }
}

impl TryFrom<String> for ColorValue {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        ColorValue::parse(&value)
    }
}

impl From<ColorValue> for String {
    fn from(value: ColorValue) -> Self {
        match value {
            ColorValue::Token(t) => t.as_str().to_string(),
            ColorValue::Hex(h) => h,
        }
    }
}

macro_rules! keyword_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident => $text:literal $(| $alias:literal)*),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum $name {
            $(#[serde(rename = $text)] $variant),+
        }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            pub fn parse(raw: &str) -> Option<$name> {
                let key = normalize_key(raw);
                $(
                    if key == normalize_key($text) $(|| key == normalize_key($alias))* {
                        return Some($name::$variant);
                    }
                )+
                None
            }

            pub fn as_str(self) -> &'static str {
                match self {
                    $($name::$variant => $text),+
                }
            }

            /// Every accepted spelling, for diagnostics and the designer property schema.
            pub fn names() -> Vec<&'static str> {
                vec![$($text),+]
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

keyword_enum!(
    /// NAV/BC `Style` values. Each maps to a theme colour + weight in the page kit.
    Emphasis {
        Standard => "Standard" | "none",
        StandardAccent => "StandardAccent",
        Strong => "Strong" | "bold",
        StrongAccent => "StrongAccent",
        Attention => "Attention",
        AttentionAccent => "AttentionAccent",
        Favorable => "Favorable",
        Unfavorable => "Unfavorable",
        Ambiguous => "Ambiguous",
        Subordinate => "Subordinate",
    }
);

keyword_enum!(Radius { None => "none", Sm => "sm", Md => "md", Lg => "lg", Full => "full" | "pill" });
keyword_enum!(Shadow { None => "none", Sm => "sm", Md => "md", Lg => "lg" });
keyword_enum!(Density { Compact => "compact" | "sm", Comfortable => "comfortable" | "md", Spacious => "spacious" | "lg" });
keyword_enum!(Breakpoint { Xs => "xs", Sm => "sm", Md => "md", Lg => "lg", Xl => "xl" });
keyword_enum!(
    /// NAV `Importance`: Promoted fields show in collapsed FastTabs, Additional ones only on "Show more".
    Importance { Promoted => "Promoted", Standard => "Standard", Additional => "Additional" }
);
keyword_enum!(
    /// Action bar areas, as in BC.
    ActionArea {
        Processing => "Processing",
        Navigation => "Navigation",
        Reporting => "Reporting",
        Creation => "Creation" | "New",
        Promoted => "Promoted",
    }
);
keyword_enum!(
    /// NAV `ExtendedDatatype`: refines how a text/number field is edited and displayed.
    ExtendedDatatype {
        Email => "Email" | "EMail",
        Phone => "Phone" | "PhoneNo",
        Url => "Url" | "URL" | "Link",
        Masked => "Masked" | "Password",
        Ratio => "Ratio" | "Progress",
        Barcode => "Barcode",
        Person => "Person",
        Color => "Color" | "Colour",
    }
);
keyword_enum!(PageOpenMode { View => "View", Edit => "Edit", Create => "Create" });
keyword_enum!(ActionSelection { None => "none", Single => "single", Multiple => "multiple" | "multi" });

/// Visual style of a page, control or action. Only design tokens and validated hex colours; never raw CSS.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageStyleIR {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<ColorValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_color: Option<ColorValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_color: Option<ColorValue>,
    /// Border width in pixels, 0..=4.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_width: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<Radius>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<Shadow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub density: Option<Density>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emphasis: Option<Emphasis>,
    /// Lucide icon name in kebab-case (`receipt`, `truck`, `user-minus`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

impl PageStyleIR {
    pub const MAX_BORDER_WIDTH: u8 = 4;

    pub fn is_empty(&self) -> bool {
        self == &PageStyleIR::default()
    }

    /// Overlays every property set in `other` on top of `self` (extension `modify` semantics).
    pub fn overlay(&mut self, other: &PageStyleIR) {
        if other.background.is_some() {
            self.background = other.background.clone();
        }
        if other.text_color.is_some() {
            self.text_color = other.text_color.clone();
        }
        if other.border_color.is_some() {
            self.border_color = other.border_color.clone();
        }
        if other.border_width.is_some() {
            self.border_width = other.border_width;
        }
        if other.radius.is_some() {
            self.radius = other.radius;
        }
        if other.shadow.is_some() {
            self.shadow = other.shadow;
        }
        if other.density.is_some() {
            self.density = other.density;
        }
        if other.emphasis.is_some() {
            self.emphasis = other.emphasis;
        }
        if other.icon.is_some() {
            self.icon = other.icon.clone();
        }
    }
}

/// Normalizes an authored icon name (`UserMinus`, `user_minus`, `user-minus`) to lucide kebab-case.
/// Returns `None` when the name contains characters lucide names never use.
pub fn normalize_icon_name(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || !trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return None;
    }
    let mut out = String::with_capacity(trimmed.len() + 4);
    let chars: Vec<char> = trimmed.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c == '_' || c == '-' {
            if !out.ends_with('-') && !out.is_empty() {
                out.push('-');
            }
            continue;
        }
        if c.is_ascii_uppercase() {
            let prev = if i > 0 { Some(chars[i - 1]) } else { None };
            let next = chars.get(i + 1).copied();
            let boundary = match prev {
                Some(p) if p.is_ascii_lowercase() || p.is_ascii_digit() => true,
                Some(p) if p.is_ascii_uppercase() => next.is_some_and(|n| n.is_ascii_lowercase()),
                _ => false,
            };
            if boundary && !out.ends_with('-') {
                out.push('-');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() { None } else { Some(out) }
}

/// Column span per breakpoint on a 12-column grid.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BreakpointSpans {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xs: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sm: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub md: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lg: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xl: Option<u8>,
}

impl BreakpointSpans {
    pub const COLUMNS: u8 = 12;

    pub fn set(&mut self, bp: Breakpoint, span: u8) {
        let slot = match bp {
            Breakpoint::Xs => &mut self.xs,
            Breakpoint::Sm => &mut self.sm,
            Breakpoint::Md => &mut self.md,
            Breakpoint::Lg => &mut self.lg,
            Breakpoint::Xl => &mut self.xl,
        };
        *slot = Some(span);
    }

    pub fn is_empty(&self) -> bool {
        self == &BreakpointSpans::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponsiveIR {
    #[serde(default, skip_serializing_if = "BreakpointSpans::is_empty")]
    pub span: BreakpointSpans,
    /// The control is not rendered below this breakpoint (e.g. a secondary column on phones).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hide_below: Option<Breakpoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub importance: Option<Importance>,
}

impl ResponsiveIR {
    pub fn is_empty(&self) -> bool {
        self == &ResponsiveIR::default()
    }

    pub fn overlay(&mut self, other: &ResponsiveIR) {
        for (bp, v) in [
            (Breakpoint::Xs, other.span.xs),
            (Breakpoint::Sm, other.span.sm),
            (Breakpoint::Md, other.span.md),
            (Breakpoint::Lg, other.span.lg),
            (Breakpoint::Xl, other.span.xl),
        ] {
            if let Some(v) = v {
                self.span.set(bp, v);
            }
        }
        if other.hide_below.is_some() {
            self.hide_below = other.hide_below;
        }
        if other.importance.is_some() {
            self.importance = other.importance;
        }
    }
}

// ----------------------------------------------------
// AUTHORED PAGE IR
// ----------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageViewIR {
    pub name: String,
    #[serde(default)]
    pub caption: Option<String>,
    #[serde(default)]
    pub filter: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub is_default: bool,
}

/// A page-level variable (NAV page globals): bound by `field(name)` when no table column has that name.
/// Lets dialogs such as a sign-in page collect values that are not stored on the source table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageVariableIR {
    pub name: String,
    /// BOS data type text, e.g. `String(128)`, `Boolean`, `Decimal(18, 2)`.
    pub data_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionGroupIR {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area: Option<ActionArea>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default = "default_true", skip_serializing_if = "Clone::clone")]
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageIR {
    #[serde(default = "legacy_ir_version")]
    pub ir_version: u32,
    pub name: String,
    pub caption: Option<String>,
    pub page_type: String,
    pub source_table: Option<String>,
    #[serde(default)]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub editable: Option<bool>,
    #[serde(default)]
    pub insert_allowed: Option<bool>,
    #[serde(default)]
    pub modify_allowed: Option<bool>,
    #[serde(default)]
    pub delete_allowed: Option<bool>,
    #[serde(default)]
    pub filters: Vec<String>,
    #[serde(default)]
    pub views: Vec<PageViewIR>,
    pub controls: Vec<PageControlIR>,
    pub actions: Vec<PageActionIR>,
    pub triggers: Vec<TriggerIR>,
    /// Card page opened from a list row (NAV `CardPageId`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_page: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data_caption_fields: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub refresh_on_activate: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub delayed_insert: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub multiple_new_lines: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub auto_split_key: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_filter: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage_category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructional_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<PageStyleIR>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub action_groups: Vec<ActionGroupIR>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<PageVariableIR>,
    /// Authored page properties without a typed slot, kept verbatim for extensibility.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, String>,
}

impl PageIR {
    /// Typed page type; `None` when the authored value is not a known page type.
    pub fn page_type_kind(&self) -> Option<PageType> {
        PageType::parse(&self.page_type)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageExtensionIR {
    #[serde(default = "legacy_ir_version")]
    pub ir_version: u32,
    pub name: String,
    pub target_page: String,
    pub controls: Vec<PageControlIR>,
    pub actions: Vec<PageActionIR>,
    pub triggers: Vec<TriggerIR>,
    /// Overrides the base page caption when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    /// Machine-owned super-user customization layer: merged last, tolerant to missing anchors,
    /// restricted to the customization directive whitelist.
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_customization: bool,
    /// Customization scope. `None` = everyone using this database.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<PageStyleIR>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub action_groups: Vec<ActionGroupIR>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<PageVariableIR>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageControlIR {
    pub kind: String,
    pub name: String,
    pub source_expr: Option<String>,
    pub caption: Option<String>,
    #[serde(default)]
    pub properties: BTreeMap<String, String>,
    pub children: Vec<PageControlIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<PageStyleIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub responsive: Option<ResponsiveIR>,
    /// Control triggers (`onValidate`, `onLookup`, `onDrillDown`, `onAssistEdit`); executed server-side.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triggers: Vec<TriggerIR>,
}

impl PageControlIR {
    pub fn new(kind: impl Into<String>, name: impl Into<String>) -> PageControlIR {
        PageControlIR {
            kind: kind.into(),
            name: name.into(),
            source_expr: None,
            caption: None,
            properties: BTreeMap::new(),
            children: Vec::new(),
            style: None,
            responsive: None,
            triggers: Vec::new(),
        }
    }

    pub fn kind_typed(&self) -> PageControlKind {
        PageControlKind::parse(&self.kind)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageActionIR {
    pub name: String,
    pub caption: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub shortcut: Option<String>,
    #[serde(default)]
    pub confirm: Option<String>,
    #[serde(default)]
    pub show_when: Option<String>,
    #[serde(default)]
    pub enable_when: Option<String>,
    #[serde(default)]
    pub run_codeunit: Option<String>,
    #[serde(default)]
    pub open_page: Option<String>,
    #[serde(default)]
    pub selection: Option<String>,
    pub script: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area: Option<ActionArea>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub promoted_only: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub promoted_category: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_default: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub in_footer_bar: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub ellipsis: bool,
    #[serde(default = "default_true", skip_serializing_if = "Clone::clone")]
    pub visible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_page_mode: Option<PageOpenMode>,
    /// `openPage` filters: target field -> client expression evaluated on the current record.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub run_page_link: BTreeMap<String, String>,
    /// `actionref(Name; Target)`: a promoted reference to another action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Extension placement: `addFirst`, `addLast`, `addBefore`, `addAfter`, `modify`, `hide`,
    /// `moveBefore`, `moveAfter`. `None` on base pages and for plain added/overridden actions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<String>,
    /// Anchor of `placement` (action, group or area name).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<PageStyleIR>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, String>,
}

impl PageActionIR {
    pub fn new(name: impl Into<String>) -> PageActionIR {
        PageActionIR {
            name: name.into(),
            caption: None,
            icon: None,
            primary: false,
            shortcut: None,
            confirm: None,
            show_when: None,
            enable_when: None,
            run_codeunit: None,
            open_page: None,
            selection: None,
            script: None,
            area: None,
            group: None,
            tooltip: None,
            promoted_only: false,
            promoted_category: None,
            is_default: false,
            in_footer_bar: false,
            ellipsis: false,
            visible: true,
            run_page_mode: None,
            run_page_link: BTreeMap::new(),
            target: None,
            placement: None,
            anchor: None,
            style: None,
            properties: BTreeMap::new(),
        }
    }
}

// ----------------------------------------------------
// RUNTIME-RESOLVED PAGE (legacy JSON API)
// ----------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedFieldMetaIR {
    pub name: String,
    #[serde(default)]
    pub caption: Option<String>,
    pub data_type: String,
    #[serde(default)]
    pub not_null: bool,
    #[serde(default)]
    pub is_primary_key: bool,
    #[serde(default)]
    pub default_value: Option<String>,
    #[serde(default)]
    pub relation_table: Option<String>,
    #[serde(default)]
    pub relation_field: Option<String>,
    #[serde(default)]
    pub is_flow_field: bool,
    #[serde(default)]
    pub flow_field_expr: Option<String>,
    #[serde(default = "default_true")]
    pub editable: bool,
    #[serde(default = "default_true")]
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedPageIR {
    pub page: PageIR,
    #[serde(default)]
    pub resolved_fields: BTreeMap<String, ResolvedFieldMetaIR>,
    #[serde(default)]
    pub state_machine: Option<String>,
    #[serde(default)]
    pub initial_state: Option<String>,
    #[serde(default)]
    pub transitions: Vec<TransitionIR>,
}

// ----------------------------------------------------
// COMPILED PAGE IR (input of the Svelte code generator)
// ----------------------------------------------------

/// A client-side expression compiled from the whitelisted page expression language.
/// `js` reads only `rec[...]`, `page[...]` and the page-kit helper object `fx`; never `eval`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientExprIR {
    /// Canonical source text (round-trips through the expression parser).
    pub source: String,
    pub js: String,
    /// SQL names of record fields read by the expression (fetch & reactivity dependencies).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<String>,
    /// Page variables read by the expression.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FieldControl {
    Text,
    TextArea,
    Number,
    Checkbox,
    Switch,
    Date,
    DateTime,
    Time,
    Select,
    Lookup,
    Masked,
    Email,
    Phone,
    Url,
    Color,
    Json,
    Image,
    ReadOnly,
}

impl FieldControl {
    pub const ALL: [FieldControl; 18] = [
        FieldControl::Text,
        FieldControl::TextArea,
        FieldControl::Number,
        FieldControl::Checkbox,
        FieldControl::Switch,
        FieldControl::Date,
        FieldControl::DateTime,
        FieldControl::Time,
        FieldControl::Select,
        FieldControl::Lookup,
        FieldControl::Masked,
        FieldControl::Email,
        FieldControl::Phone,
        FieldControl::Url,
        FieldControl::Color,
        FieldControl::Json,
        FieldControl::Image,
        FieldControl::ReadOnly,
    ];

    pub fn parse(raw: &str) -> Option<FieldControl> {
        Some(match normalize_key(raw).as_str() {
            "text" | "input" => FieldControl::Text,
            "textarea" | "multiline" => FieldControl::TextArea,
            "number" | "decimal" | "integer" => FieldControl::Number,
            "checkbox" => FieldControl::Checkbox,
            "switch" | "toggle" => FieldControl::Switch,
            "date" => FieldControl::Date,
            "datetime" => FieldControl::DateTime,
            "time" => FieldControl::Time,
            "select" | "option" | "dropdown" => FieldControl::Select,
            "lookup" | "combobox" => FieldControl::Lookup,
            "masked" | "password" => FieldControl::Masked,
            "email" => FieldControl::Email,
            "phone" => FieldControl::Phone,
            "url" | "link" => FieldControl::Url,
            "color" | "colour" => FieldControl::Color,
            "json" => FieldControl::Json,
            "image" | "media" => FieldControl::Image,
            "readonly" => FieldControl::ReadOnly,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            FieldControl::Text => "text",
            FieldControl::TextArea => "textArea",
            FieldControl::Number => "number",
            FieldControl::Checkbox => "checkbox",
            FieldControl::Switch => "switch",
            FieldControl::Date => "date",
            FieldControl::DateTime => "dateTime",
            FieldControl::Time => "time",
            FieldControl::Select => "select",
            FieldControl::Lookup => "lookup",
            FieldControl::Masked => "masked",
            FieldControl::Email => "email",
            FieldControl::Phone => "phone",
            FieldControl::Url => "url",
            FieldControl::Color => "color",
            FieldControl::Json => "json",
            FieldControl::Image => "image",
            FieldControl::ReadOnly => "readOnly",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FieldBinding {
    Column,
    FlowField,
    Variable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldOptionIR {
    pub value: String,
    pub caption: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LookupIR {
    /// Authored table name of the related table (as declared in `.bos`).
    pub table: String,
    pub sql_table: String,
    pub sql_field: String,
    /// Page used to pick a value (target table's lookup page, else its first List page).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
    /// Columns shown in the lookup dropdown (target field + its brick / first descriptive columns).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub display_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrillDownIR {
    pub table: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
}

/// Field binding with every NAV-style property resolved from the source table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedFieldIR {
    /// Name as authored in `field(...)`.
    pub source: String,
    pub binding: FieldBinding,
    /// Key in the record JSON (`sql_name` for columns & FlowFields, variable name for variables).
    pub key: String,
    pub data_type: String,
    pub control: FieldControl,
    pub required: bool,
    pub editable: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_primary_key: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
    /// Decimal places (`min`, `max`); NAV `DecimalPlaces = 2:5`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decimals: Option<(u8, u8)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_format: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub blank_zero: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extended_type: Option<ExtendedDatatype>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<FieldOptionIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lookup: Option<LookupIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drill_down: Option<DrillDownIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub assist_edit: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub quick_entry: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    /// Property names filled from the table instead of being authored (shown as "auto" in the designer).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub auto_filled: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubPageLinkIR {
    /// Field on the sub page's table (record key).
    pub field: String,
    pub value: ClientExprIR,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedPartIR {
    pub page: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub link: Vec<SubPageLinkIR>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub update_propagation: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedControlIR {
    /// Canonical kind (`PageControlKind::as_str`).
    pub kind: String,
    pub name: String,
    /// Stable node path, e.g. `area:Content/group:General/field:name`. Used by the designer and `data-bos-node`.
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructional_text: Option<String>,
    pub visible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<ResolvedFieldIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part: Option<ResolvedPartIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<PageStyleIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub responsive: Option<ResponsiveIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_when: Option<ClientExprIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_when: Option<ClientExprIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editable_when: Option<ClientExprIR>,
    /// Expression returning an `Emphasis` name at runtime (NAV `StyleExpr`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_expr: Option<ClientExprIR>,
    /// Names of server-side control triggers (`onValidate`, `onLookup`, ...).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triggers: Vec<String>,
    /// Remaining authored properties (layout hints such as `cols`, `gap`, `ratio`, `showCaption`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<ResolvedControlIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedActionIR {
    pub name: String,
    pub caption: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
    pub area: ActionArea,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub primary: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub promoted_only: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub promoted_category: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_default: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub in_footer_bar: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub ellipsis: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shortcut: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirm: Option<String>,
    pub selection: ActionSelection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_codeunit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_page: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_page_mode: Option<PageOpenMode>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub run_page_link: Vec<SubPageLinkIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_when: Option<ClientExprIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_when: Option<ClientExprIR>,
    /// The action has an `onAction` script; it runs server-side through the page trigger endpoint.
    #[serde(default, skip_serializing_if = "is_false")]
    pub has_script: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<PageStyleIR>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PageDiagnosticSeverity {
    Error,
    Warning,
    Info,
}

impl PageDiagnosticSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            PageDiagnosticSeverity::Error => "error",
            PageDiagnosticSeverity::Warning => "warning",
            PageDiagnosticSeverity::Info => "info",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageDiagnosticIR {
    pub severity: PageDiagnosticSeverity,
    pub code: String,
    pub message: String,
    /// Page (or extension) the diagnostic belongs to.
    pub object: String,
    /// Node path of the offending control/action, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledPageIR {
    pub ir_version: u32,
    pub name: String,
    pub caption: String,
    pub page_type: PageType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_table: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_sql_table: Option<String>,
    /// SQL names of the source table primary key.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub primary_key: Vec<String>,
    pub editable: bool,
    pub insert_allowed: bool,
    pub modify_allowed: bool,
    pub delete_allowed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_page: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data_caption_fields: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub refresh_on_activate: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub delayed_insert: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub multiple_new_lines: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub auto_split_key: bool,
    pub show_filter: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage_category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructional_text: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub views: Vec<PageViewIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<PageStyleIR>,
    pub controls: Vec<ResolvedControlIR>,
    pub actions: Vec<ResolvedActionIR>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub action_groups: Vec<ActionGroupIR>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<PageVariableIR>,
    /// Names of page-level server triggers (`onOpenPage`, `onAfterGetRecord`, ...).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triggers: Vec<String>,
    /// Extensions merged into this page, in merge order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub applied_extensions: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_v1_page_json_still_loads() {
        let v1 = r#"{
            "name": "CustomerCard", "caption": "Customer", "page_type": "Card", "source_table": "Customer",
            "controls": [{"kind": "group", "name": "General", "source_expr": null, "caption": null,
                          "properties": {"cols": "2"}, "children": []}],
            "actions": [{"name": "Post", "caption": "Post", "script": null}],
            "triggers": []
        }"#;
        let page: PageIR = serde_json::from_str(v1).expect("v1 page json");
        assert_eq!(page.ir_version, 1);
        assert_eq!(page.controls[0].properties.get("cols").map(String::as_str), Some("2"));
        assert!(page.actions[0].visible, "visible defaults to true for legacy actions");
        assert!(page.controls[0].style.is_none());

        let round = serde_json::to_string(&page).unwrap();
        let back: PageIR = serde_json::from_str(&round).unwrap();
        assert_eq!(page, back);
    }

    #[test]
    fn legacy_page_extension_json_still_loads() {
        let v1 = r#"{"name": "Ext", "target_page": "CustomerCard", "controls": [], "actions": [], "triggers": []}"#;
        let ext: PageExtensionIR = serde_json::from_str(v1).unwrap();
        assert!(!ext.is_customization);
        assert!(ext.scope.is_none());
    }

    #[test]
    fn page_type_aliases() {
        assert_eq!(PageType::parse("Record"), Some(PageType::Card));
        assert_eq!(PageType::parse("Dialog"), Some(PageType::StandardDialog));
        assert_eq!(PageType::parse("Wizard"), Some(PageType::NavigatePage));
        assert_eq!(PageType::parse("Workspace"), Some(PageType::RoleCenter));
        assert_eq!(PageType::parse("list_part"), Some(PageType::ListPart));
        assert_eq!(PageType::parse("Mystery"), None);
        for t in PageType::ALL {
            assert_eq!(PageType::parse(t.as_str()), Some(t));
        }
    }

    #[test]
    fn control_kind_roundtrip_and_aliases() {
        assert_eq!(PageControlKind::parse("lines"), PageControlKind::Repeater);
        assert_eq!(PageControlKind::parse("factbox"), PageControlKind::Part);
        assert_eq!(PageControlKind::parse("addAfter"), PageControlKind::AddAfter);
        assert!(PageControlKind::parse("moveBefore").is_directive());
        assert!(PageControlKind::parse("group").is_container());
        assert_eq!(PageControlKind::parse("weird"), PageControlKind::Other("weird".into()));
        assert!(!PageType::List.allows(PageControlKind::WizardStep));
        assert!(PageType::NavigatePage.allows(PageControlKind::WizardStep));
        assert!(PageType::RoleCenter.allows(PageControlKind::CueGroup));
        assert!(!PageType::ListPart.allows(PageControlKind::Part));
    }

    #[test]
    fn colour_values_validate_and_normalize() {
        assert_eq!(ColorValue::parse("#1E40AF").unwrap(), ColorValue::Hex("#1e40af".into()));
        assert_eq!(ColorValue::parse("#abc").unwrap(), ColorValue::Hex("#aabbcc".into()));
        assert_eq!(ColorValue::parse("warning").unwrap(), ColorValue::Token(ColorToken::Attention));
        assert!(ColorValue::parse("#12345").is_err());
        assert!(ColorValue::parse("#gggggg").is_err());
        assert!(ColorValue::parse("red; background: url(x)").is_err());

        let style = PageStyleIR {
            background: Some(ColorValue::parse("primary").unwrap()),
            border_color: Some(ColorValue::parse("#1e40af").unwrap()),
            border_width: Some(1),
            radius: Some(Radius::Md),
            emphasis: Some(Emphasis::Attention),
            icon: Some("truck".into()),
            ..PageStyleIR::default()
        };
        let json = serde_json::to_value(&style).unwrap();
        assert_eq!(json["background"], "primary");
        assert_eq!(json["borderColor"], "#1e40af");
        assert_eq!(json["radius"], "md");
        assert_eq!(json["emphasis"], "Attention");
        let back: PageStyleIR = serde_json::from_value(json).unwrap();
        assert_eq!(back, style);
        assert!(serde_json::from_value::<PageStyleIR>(serde_json::json!({"background": "javascript:x"})).is_err());
    }

    #[test]
    fn style_overlay_keeps_unset_values() {
        let mut base = PageStyleIR { radius: Some(Radius::Lg), icon: Some("user".into()), ..Default::default() };
        base.overlay(&PageStyleIR { icon: Some("truck".into()), ..Default::default() });
        assert_eq!(base.radius, Some(Radius::Lg));
        assert_eq!(base.icon.as_deref(), Some("truck"));
    }

    #[test]
    fn icon_names_normalize_to_lucide_kebab_case() {
        assert_eq!(normalize_icon_name("UserMinus").as_deref(), Some("user-minus"));
        assert_eq!(normalize_icon_name("XCircle").as_deref(), Some("x-circle"));
        assert_eq!(normalize_icon_name("LogIn").as_deref(), Some("log-in"));
        assert_eq!(normalize_icon_name("user_minus").as_deref(), Some("user-minus"));
        assert_eq!(normalize_icon_name("receipt").as_deref(), Some("receipt"));
        assert_eq!(normalize_icon_name("bar-chart-3").as_deref(), Some("bar-chart-3"));
        assert_eq!(normalize_icon_name("<svg>"), None);
    }

    #[test]
    fn keyword_enums_parse_aliases() {
        assert_eq!(Emphasis::parse("attention"), Some(Emphasis::Attention));
        assert_eq!(ExtendedDatatype::parse("password"), Some(ExtendedDatatype::Masked));
        assert_eq!(ActionArea::parse("new"), Some(ActionArea::Creation));
        assert_eq!(Radius::parse("pill"), Some(Radius::Full));
        assert_eq!(Importance::parse("additional"), Some(Importance::Additional));
        assert_eq!(serde_json::to_value(ActionArea::Processing).unwrap(), "Processing");
    }
}
