/**
 * `@bos/page-kit`: the runtime every compiled BOS page imports.
 * Built once into the app bundle and shared with server-compiled page modules through the import map,
 * so pages, the shell and the Studio use one Svelte runtime and one set of stores.
 */
import { themeStore } from "$lib/bos-ui/theme/index.js";
import { setWorkDateProvider } from "./fx.js";

// `fx.workDate()` follows the user's work date (NAV WorkDate) chosen in the shell.
setWorkDateProvider(() => themeStore.workDate);

export { createPage, PageController } from "./page.svelte.js";
export { fx, setWorkDateProvider } from "./fx.js";
export { pageManifest, loadPageComponent, loadPreview } from "./loader.svelte.js";
export { ICON_NAMES, hasIcon, loadIcon } from "./icons.js";
export { registerUserControl, userControl, userControlNames } from "./user-controls.js";

export { default as PageFrame } from "./PageFrame.svelte";
export { default as Area } from "./Area.svelte";
export { default as FactBoxes } from "./FactBoxes.svelte";
export { default as Group } from "./Group.svelte";
export { default as Grid } from "./Grid.svelte";
export { default as Card } from "./Card.svelte";
export { default as Flex } from "./Flex.svelte";
export { default as Container } from "./Container.svelte";
export { default as CueGroup } from "./CueGroup.svelte";
export { default as Cue } from "./Cue.svelte";
export { default as Field } from "./Field.svelte";
export { default as Label } from "./Label.svelte";
export { default as Image } from "./Image.svelte";
export { default as Spacer } from "./Spacer.svelte";
export { default as Divider } from "./Divider.svelte";
export { default as Headline } from "./Headline.svelte";
export { default as UserControl } from "./UserControl.svelte";
export { default as Part } from "./Part.svelte";
export { default as SystemPart } from "./SystemPart.svelte";
export { default as Repeater } from "./Repeater.svelte";
export { default as Tabs } from "./Tabs.svelte";
export { default as Split } from "./Split.svelte";
export { default as WizardSteps } from "./WizardSteps.svelte";
export { default as Icon } from "./Icon.svelte";
export { default as PageHost } from "./PageHost.svelte";
