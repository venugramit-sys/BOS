<script>
  import { shell } from "./shellStore.svelte.js";
  import {
    LayoutDashboard,
    Sparkles,
    Workflow,
    Columns3,
    Kanban,
    ReceiptText,
    History,
    Settings,
    KeyRound,
    ShieldCheck,
    Fingerprint,
    Building2,
    Lock,
    Compass,
    ExternalLink,
    ArrowRight,
    CheckCircle2,
    Layers,
    MonitorDot,
    Sliders
  } from "lucide-svelte";
  import { Badge } from "$lib/bos-ui/badge/index.js";

  const MODERN_TEMPLATES = [
    {
      id: "executive-dashboard",
      pageName: "ExecutiveDashboardTemplate",
      category: "Operations & Leadership",
      title: "Executive Operations Cockpit",
      pageType: "RoleCenter",
      icon: LayoutDashboard,
      accent: "from-blue-600 to-indigo-600",
      description: "C-Suite & operations cockpit featuring real-time revenue telemetry, cue card queues, growth velocity, and quick action bar.",
      highlights: ["Real-time revenue telemetry", "Cue group queue counters", "Direct document launch triggers"]
    },
    {
      id: "setup-wizard",
      pageName: "SetupWizardTemplate",
      category: "Onboarding & Provisioning",
      title: "Assisted Setup & Provisioning Wizard",
      pageType: "NavigatePage",
      icon: Workflow,
      accent: "from-emerald-600 to-teal-600",
      description: "Step-by-step guided onboarding workflow for entity configuration, fiscal calendars, and India DPDP Act / GDPR governance.",
      highlights: ["4-step guided progression", "Per-step input validation", "Automatic schema configuration"]
    },
    {
      id: "split-worklist",
      pageName: "SplitMasterDetailWorklist",
      category: "High-Speed Operations",
      title: "Master-Detail Split Worklist",
      pageType: "NavigatePage",
      icon: Columns3,
      accent: "from-violet-600 to-purple-600",
      description: "Linear / Zendesk / Outlook style 2-column split view for rapid document review without disruptive full-page navigation.",
      highlights: ["Dual-pane synchronized layout", "1-click status transitions", "Fast queue triage"]
    },
    {
      id: "kanban-pipeline",
      pageName: "KanbanPipelineTemplate",
      category: "Visual Workflows",
      title: "Sales & Order Pipeline Kanban Board",
      pageType: "NavigatePage",
      icon: Kanban,
      accent: "from-amber-600 to-orange-600",
      description: "Multi-stage visual pipeline tracking deal progression across discovery, review, negotiation, and win milestones.",
      highlights: ["Stage volume metrics", "Visual milestone tracking", "Instant deal status progression"]
    },
    {
      id: "document-checkout",
      pageName: "DocumentCheckoutTemplate",
      category: "High-Speed Operations",
      title: "High-Speed Document Order & Checkout",
      pageType: "Document",
      icon: ReceiptText,
      accent: "from-sky-600 to-cyan-600",
      description: "FastTabs document entry with sticky totals summary, real-time cascading discounts, tax calculations, and 1-click posting.",
      highlights: ["Exact decimal precision math", "Sticky totals summary sidebar", "Two-phase ledger posting trigger"]
    },
    {
      id: "audit-timeline",
      pageName: "AuditComplianceTimeline",
      category: "Governance & Security",
      title: "Compliance & Security Audit Timeline",
      pageType: "NavigatePage",
      icon: History,
      accent: "from-rose-600 to-pink-600",
      description: "Chronological immutable ledger and security event timeline with cryptographic SHA-256 hash verification and state diff snapshots.",
      highlights: ["Zero hash drift verification", "Actor & IP tracing", "xRec vs Rec diff inspection"]
    },
    {
      id: "enterprise-settings",
      pageName: "EnterpriseSettingsCenter",
      category: "Administration",
      title: "Enterprise Settings & Preferences Hub",
      pageType: "Card",
      icon: Settings,
      accent: "from-slate-700 to-slate-900",
      description: "Comprehensive singleton configuration center for security policies, sequential numbering series, and webhook integrations.",
      highlights: ["FastTabs categorization", "Password policy rules", "Webhook gateway test ping"]
    }
  ];

  const LOGIN_TEMPLATES = [
    {
      id: "split-hero-login",
      pageName: "SplitHeroLogin",
      title: "Split Hero Enterprise Login",
      icon: Building2,
      model: "Salesforce / WorkOS",
      description: "Two-column corporate showcase with trust certifications (SOC2, ISO27001, DPDP Act 2023) and SAML 2.0 SSO."
    },
    {
      id: "glassmorphism-login",
      pageName: "GlassmorphismLogin",
      title: "Glassmorphism Minimalist",
      icon: Sparkles,
      model: "Linear / Stripe",
      description: "Frosted-glass backdrop with modern typography, security shield cues, and magic link request option."
    },
    {
      id: "multi-tenant-login",
      pageName: "MultiTenantDomainLogin",
      title: "Multi-Tenant Workspace Router",
      icon: Compass,
      model: "Slack / Notion",
      description: "Subdomain and organization slug router dynamically routing users to their isolated tenant database."
    },
    {
      id: "mfa-otp-login",
      pageName: "MfaOtpStepUpLogin",
      title: "MFA TOTP Step-Up Security",
      icon: ShieldCheck,
      model: "AWS IAM / GitHub",
      description: "Step-up two-factor authentication requiring time-based one-time passcodes (RFC 6238) or backup keys."
    },
    {
      id: "kiosk-pin-login",
      pageName: "KioskPinLockscreen",
      title: "POS Kiosk Numeric PIN Pad",
      icon: Lock,
      model: "Square POS / Terminal",
      description: "Touchscreen-optimized 10-key numeric keypad for point-of-sale registers and warehouse terminals."
    },
    {
      id: "passkey-login",
      pageName: "PasskeyWebAuthnLogin",
      title: "FIDO2 Passkey Passwordless",
      icon: Fingerprint,
      model: "Apple ID / Clerk",
      description: "Modern biometric passwordless sign-in utilizing WebAuthn hardware keys, TouchID, and FaceID."
    }
  ];

  function openTemplateTab(t) {
    shell.openTab({
      id: `template-${t.pageName.toLowerCase()}`,
      title: t.title,
      type: "page",
      name: t.pageName,
      closable: true
    });
  }
</script>

<div class="space-y-8 pb-12">
  <!-- Hero Section -->
  <div class="relative overflow-hidden rounded-2xl bg-gradient-to-br from-indigo-950 via-slate-900 to-slate-950 border border-indigo-900/40 p-6 sm:p-8 text-white shadow-xl">
    <div class="absolute -right-12 -top-12 size-64 bg-indigo-500/10 rounded-full blur-3xl pointer-events-none"></div>
    <div class="relative z-10 max-w-2xl space-y-3">
      <div class="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-indigo-500/20 border border-indigo-400/30 text-indigo-300 text-xs font-semibold">
        <Sparkles class="size-3.5" />
        <span>BOS Enterprise Ready-Made Templates Gallery</span>
      </div>
      <h1 class="text-2xl sm:text-3xl font-extrabold tracking-tight text-white">
        Gold-Standard Modern Page Templates
      </h1>
      <p class="text-xs sm:text-sm text-slate-300 leading-relaxed">
        Explore production-ready meta-objects demonstrating everything BOS provides alongside gold-standard UI, UX, and operational patterns from market leaders like Microsoft Dynamics 365, Salesforce, Linear, and Stripe.
      </p>
    </div>
  </div>

  <!-- Operational & Leadership Templates -->
  <div class="space-y-4">
    <div class="flex items-center justify-between pb-2 border-b border-slate-200 dark:border-slate-800">
      <div>
        <h2 class="text-base font-bold text-slate-900 dark:text-slate-100 flex items-center gap-2">
          <Layers class="size-4.5 text-indigo-600 dark:text-indigo-400" />
          <span>Core Enterprise Operational Templates</span>
        </h2>
        <p class="text-xs text-slate-500 dark:text-slate-400 mt-0.5">
          Fully compiled meta-objects ready to launch directly into your active workspace.
        </p>
      </div>
      <span class="text-xs font-medium text-slate-400">{MODERN_TEMPLATES.length} Templates</span>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      {#each MODERN_TEMPLATES as template}
        <div class="group relative flex flex-col justify-between rounded-xl border border-slate-200/90 dark:border-slate-800/90 bg-white dark:bg-slate-900 p-5 shadow-xs hover:shadow-md hover:border-indigo-400/60 dark:hover:border-indigo-500/40 transition-all">
          <div class="space-y-3">
            <div class="flex items-center justify-between gap-2">
              <div class="size-9 rounded-lg bg-gradient-to-br {template.accent} flex items-center justify-center text-white shadow-xs">
                <template.icon class="size-4.5" />
              </div>
              <span class="px-2 py-0.5 rounded text-[10px] font-mono font-medium bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-400 border border-slate-200 dark:border-slate-700">
                {template.pageType}
              </span>
            </div>

            <div>
              <div class="text-[11px] font-semibold text-indigo-600 dark:text-indigo-400 uppercase tracking-wider">
                {template.category}
              </div>
              <h3 class="text-sm font-bold text-slate-900 dark:text-slate-100 mt-0.5 group-hover:text-indigo-600 dark:group-hover:text-indigo-400 transition-colors">
                {template.title}
              </h3>
              <p class="text-xs text-slate-500 dark:text-slate-400 mt-1 line-clamp-2 leading-normal">
                {template.description}
              </p>
            </div>

            <div class="pt-2 border-t border-slate-100 dark:border-slate-800/60 space-y-1">
              {#each template.highlights as h}
                <div class="flex items-center gap-1.5 text-[11px] text-slate-600 dark:text-slate-400">
                  <CheckCircle2 class="size-3 text-emerald-500 shrink-0" />
                  <span class="truncate">{h}</span>
                </div>
              {/each}
            </div>
          </div>

          <button
            type="button"
            onclick={() => openTemplateTab(template)}
            class="mt-4 w-full h-8 rounded-lg bg-slate-50 hover:bg-indigo-600 text-slate-700 hover:text-white dark:bg-slate-800 dark:hover:bg-indigo-600 dark:text-slate-200 dark:hover:text-white text-xs font-medium flex items-center justify-center gap-1.5 transition-all cursor-pointer border border-slate-200/80 dark:border-slate-700/80 hover:border-transparent"
          >
            <span>Launch Template</span>
            <ArrowRight class="size-3.5 group-hover:translate-x-0.5 transition-transform" />
          </button>
        </div>
      {/each}
    </div>
  </div>

  <!-- Authentication Suite Templates -->
  <div class="space-y-4 pt-4">
    <div class="flex items-center justify-between pb-2 border-b border-slate-200 dark:border-slate-800">
      <div>
        <h2 class="text-base font-bold text-slate-900 dark:text-slate-100 flex items-center gap-2">
          <KeyRound class="size-4.5 text-indigo-600 dark:text-indigo-400" />
          <span>Modern Authentication & Entry Templates</span>
        </h2>
        <p class="text-xs text-slate-500 dark:text-slate-400 mt-0.5">
          Enterprise login paradigms supporting SAML SSO, Passkeys, Multi-Tenant Domain routing, and Kiosk PINs.
        </p>
      </div>
      <span class="text-xs font-medium text-slate-400">{LOGIN_TEMPLATES.length} Templates</span>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      {#each LOGIN_TEMPLATES as loginTpl}
        <div class="flex flex-col justify-between rounded-xl border border-slate-200/90 dark:border-slate-800/90 bg-white dark:bg-slate-900 p-5 shadow-xs hover:border-indigo-400/60 dark:hover:border-indigo-500/40 transition-all">
          <div class="space-y-2.5">
            <div class="flex items-center justify-between">
              <div class="size-8 rounded-lg bg-indigo-50 dark:bg-indigo-950/60 text-indigo-600 dark:text-indigo-400 flex items-center justify-center">
                <loginTpl.icon class="size-4" />
              </div>
              <span class="px-2 py-0.5 rounded text-[10px] font-medium bg-indigo-50 dark:bg-indigo-950/40 text-indigo-600 dark:text-indigo-400 border border-indigo-200 dark:border-indigo-900/60">
                {loginTpl.model}
              </span>
            </div>

            <div>
              <h3 class="text-sm font-bold text-slate-900 dark:text-slate-100">
                {loginTpl.title}
              </h3>
              <p class="text-xs text-slate-500 dark:text-slate-400 mt-1 line-clamp-2 leading-normal">
                {loginTpl.description}
              </p>
            </div>
          </div>

          <button
            type="button"
            onclick={() => openTemplateTab(loginTpl)}
            class="mt-4 w-full h-8 rounded-lg bg-slate-50 hover:bg-slate-100 dark:bg-slate-800 dark:hover:bg-slate-700 text-slate-700 dark:text-slate-200 text-xs font-medium flex items-center justify-center gap-1.5 transition-colors cursor-pointer border border-slate-200/80 dark:border-slate-700/80"
          >
            <span>Preview in Workspace</span>
            <ExternalLink class="size-3" />
          </button>
        </div>
      {/each}
    </div>
  </div>
</div>
