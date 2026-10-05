import { usePosStore } from "../stores/pos";

/** Something with a name in the UI language and, when known, its
 * French and Chinese names (categories, products, cart lines). */
export interface Named {
  name: string;
  name_fr?: string | null;
  name_zh?: string | null;
  /** Menu code ("A2"): products and cart lines from a catalog that has one. */
  code?: string | null;
}

export interface BilingualName {
  primary: string;
  /** The other language, smaller underneath; null when it adds nothing. */
  secondary: string | null;
  /** `primary` is the menu code (Settings: products shown by code). */
  isCode: boolean;
}

/**
 * Product names are shown in two languages (UX guide §10): the UI
 * language first, then French or Chinese underneath, so French- and
 * Chinese-speaking staff both read the same button.
 *
 * With products shown by code, a product that has one leads with it
 * and keeps its name in the UI language underneath.
 */
export function useBilingualName() {
  const pos = usePosStore();

  return function bilingual(item: Named): BilingualName {
    const zh = pos.language === "zh";
    const primary = zh ? (item.name_zh ?? item.name) : item.name;
    const code = item.code?.trim();
    if (pos.productLabel === "code" && code) {
      return { primary: code, secondary: primary, isCode: true };
    }
    const other = zh ? (item.name_fr ?? null) : (item.name_zh ?? null);
    return { primary, secondary: other && other !== primary ? other : null, isCode: false };
  };
}
