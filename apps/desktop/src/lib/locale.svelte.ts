// The dashboard's current language as Svelte state. Components call `t`;
// reading the language inside it makes every text re-render on a switch.
import { initialLang, loadLang, saveLang, translate, type Lang, type MessageKey, type Params } from './i18n';

// Touching `localStorage` itself can throw when site data is blocked.
const storage = () => window.localStorage;

let current = $state<Lang>(
  initialLang(
    typeof window === 'undefined' ? null : loadLang(storage),
    typeof navigator === 'undefined' ? [] : navigator.languages?.length ? navigator.languages : [navigator.language],
  ),
);

function syncDocument() {
  if (typeof document !== 'undefined') document.documentElement.lang = current;
}
syncDocument();

export const locale = {
  get lang(): Lang {
    return current;
  },
};

export function setLang(lang: Lang) {
  current = lang;
  saveLang(storage, lang);
  syncDocument();
}

export function t(key: MessageKey, params?: Params): string {
  return translate(current, key, params);
}
