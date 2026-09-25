// Build-time feature flags.
//
// `VITE_FEATURE_HUBRISE=false` is set by scripts/build_and_hash.sh for
// the certification build: the HubRise (webshop) route, views, header
// status, settings panel and order chime are then dead code and Vite
// drops them from the bundle. Any other value (including unset) keeps
// HubRise available - dev default.
export const HUBRISE_ENABLED = import.meta.env.VITE_FEATURE_HUBRISE !== "false";
