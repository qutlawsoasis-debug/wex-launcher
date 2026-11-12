/** Locale-aware formatting for sizes and dates shown in the shell. */
import { i18n, t } from '../i18n';

function num(n: number, digits: number): string {
  return new Intl.NumberFormat(i18n.locale, { maximumFractionDigits: digits, minimumFractionDigits: digits }).format(n);
}

/** "34 МБ", "4,1 ГБ", "820 КБ". */
export function formatSize(bytes: number | bigint): string {
  const b = Number(bytes);
  const kb = b / 1024;
  if (kb < 1024) return t('format.kb', { n: num(Math.max(1, kb), 0) });
  const mb = kb / 1024;
  if (mb < 1024) return t('shell.mb', { n: num(mb, mb < 10 ? 1 : 0) });
  return t('shell.gb', { n: num(mb / 1024, 1) });
}

/** "12 ноября" this year, "12 ноября 2025" otherwise. Accepts ms, seconds or an ISO string. */
export function formatDate(value: number | bigint | string | null | undefined): string {
  if (value === null || value === undefined || value === '') return '';
  let ms: number;
  if (typeof value === 'string') ms = Date.parse(value);
  else {
    const n = Number(value);
    ms = n < 1e11 ? n * 1000 : n;
  }
  if (!Number.isFinite(ms) || ms <= 0) return '';
  const d = new Date(ms);
  const sameYear = d.getFullYear() === new Date().getFullYear();
  return new Intl.DateTimeFormat(i18n.locale, sameYear ? { day: 'numeric', month: 'long' } : { day: 'numeric', month: 'long', year: 'numeric' }).format(d);
}

/** "сегодня", "вчера", "3 дня назад", "на прошлой неделе" within a month (as the reference writes it), a date after that. */
export function formatAgo(value: number | bigint | string | null | undefined): string {
  if (value === null || value === undefined || value === '') return '';
  const ms = typeof value === 'string' ? Date.parse(value) : Number(value) < 1e11 ? Number(value) * 1000 : Number(value);
  if (!Number.isFinite(ms) || ms <= 0) return '';
  const startOf = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  const days = Math.round((startOf(new Date()) - startOf(new Date(ms))) / 86_400_000);
  if (days < 0 || days > 30) return formatDate(ms);
  const rtf = new Intl.RelativeTimeFormat(i18n.locale, { numeric: 'auto' });
  return days < 7 ? rtf.format(-days, 'day') : rtf.format(-Math.floor(days / 7), 'week');
}
