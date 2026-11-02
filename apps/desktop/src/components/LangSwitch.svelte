<script lang="ts">
  import type { Lang } from '../lib/i18n';
  import { locale, setLang } from '../lib/locale.svelte';

  // Each choice is named in its own language, and so is the group, so a
  // person who reads only one of them can still find it.
  const choices: { lang: Lang; label: string }[] = [
    { lang: 'en', label: 'EN' },
    { lang: 'ko', label: '한국어' },
  ];
</script>

<div class="switch" role="group" aria-label="Language / 언어">
  {#each choices as c (c.lang)}
    <button type="button" lang={c.lang} aria-pressed={locale.lang === c.lang} onclick={() => setLang(c.lang)}>
      {c.label}
    </button>
  {/each}
</div>

<style>
  .switch {
    display: inline-flex;
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
    background: var(--surface);
  }
  button {
    border: 0;
    background: transparent;
    padding: 6px 12px;
    font-size: 13px;
    color: var(--ink-2);
  }
  button + button {
    border-left: 1px solid var(--border);
  }
  button:hover:not([aria-pressed='true']) {
    background: var(--grid);
  }
  /* The chosen side: filled and bold, not colour alone. */
  button[aria-pressed='true'] {
    background: var(--ink);
    color: var(--page);
    font-weight: 700;
  }
  button:focus-visible {
    outline-offset: -2px;
  }
</style>
