/// <reference types="vite/client" />

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<
    Record<string, unknown>,
    Record<string, unknown>,
    unknown
  >;
  export default component;
}

// Build-time feature flag, a literal set by vite.config.ts `define`.
// `VITE_FEATURE_HUBRISE=false` (scripts/build_and_hash.sh, the
// certification build) makes it `false`: the HubRise (webshop) route,
// views, settings panel, status light and order chime are then dead
// code that the bundler drops. Any other value (including unset) keeps
// HubRise available, the dev default. Read it directly where it is
// used, never through an exported constant: Rolldown does not fold an
// imported constant before emitting chunks, and Vue templates wrap an
// imported binding in `unref()`, so the webshop code would ship.
declare const __HUBRISE_ENABLED__: boolean;
