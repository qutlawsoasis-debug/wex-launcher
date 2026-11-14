/**
 * Plural forms for counts ("1 мод", "2 мода", "5 модов"). A plural key has four
 * entries in each dictionary: <base>_one, <base>_few, <base>_many, <base>_other
 * (English uses the same text for few, many and other). {n} is the count.
 */

import { i18n, t } from './i18n.svelte';

const rules = new Map<string, Intl.PluralRules>();

function category(n: number): string {
  const locale = i18n.locale;
  let r = rules.get(locale);
  if (!r) {
    r = new Intl.PluralRules(locale);
    rules.set(locale, r);
  }
  return r.select(n);
}

export function tn(base: string, n: number, params: Record<string, string | number> = {}): string {
  const cat = category(n);
  const key = ['one', 'few', 'many'].includes(cat) ? `${base}_${cat}` : `${base}_other`;
  return t(key, { n, ...params });
}
