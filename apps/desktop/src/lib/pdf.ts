// Reading a document's text in the dashboard, so only text (not the file)
// goes to the master. pdf.js is loaded only when a PDF is opened.

/** Mirrors `MAX_DOCUMENT_CHARS` in nodeloom-master's summarize.rs. */
export const MAX_DOCUMENT_CHARS = 200_000;
/** Mirrors `PART_CHARS`: about how long one part is. */
export const PART_CHARS = 2500;
/** Files larger than this are not read at all. */
export const MAX_FILE_BYTES = 50 * 1024 * 1024;

/** What pdf.js reports for a run of text on a page. */
export interface TextRun {
  str: string;
  hasEOL: boolean;
}

/**
 * One page's text from its runs. Lines become single breaks (the master
 * folds them into spaces); a word hyphenated across lines is joined.
 */
export function pageText(runs: TextRun[]): string {
  let text = '';
  for (const run of runs) {
    text += run.str;
    if (run.hasEOL) text += '\n';
  }
  return text
    .replace(/(\p{L})-\n(\p{Ll})/gu, '$1$2')
    .replace(/[ \t]+\n/g, '\n')
    .trim();
}

/** Pages become paragraphs: the master cuts parts at blank lines first. */
export function joinPages(pages: string[]): string {
  return pages.filter((p) => p.length > 0).join('\n\n');
}

/** Characters as the master counts them (code points, not UTF-16 units). */
export function countChars(text: string): number {
  let n = 0;
  for (const _ of text) n++;
  return n;
}

/** Why `text` cannot be summarized, or '' when it can. */
export function documentProblem(text: string): string {
  if (text.trim().length === 0) {
    return 'No text found. A scanned PDF is only pictures of pages; it needs text recognition (OCR) first.';
  }
  const chars = countChars(text);
  if (chars > MAX_DOCUMENT_CHARS) {
    return `The document has ${chars.toLocaleString('en-US')} characters; at most ${MAX_DOCUMENT_CHARS.toLocaleString('en-US')} can be summarized.`;
  }
  return '';
}

/** Roughly how many parts the master will cut `chars` characters into. */
export function estimateParts(chars: number): number {
  return Math.max(1, Math.ceil(chars / (PART_CHARS * 0.9)));
}

export interface ReadDocument {
  text: string;
  /** Pages in a PDF; null for plain text. */
  pages: number | null;
}

/** Reads the text of a PDF, or of a plain-text or Markdown file. */
export async function readDocument(file: File, onPage?: (done: number, total: number) => void): Promise<ReadDocument> {
  if (file.size > MAX_FILE_BYTES) {
    throw new Error(`The file is larger than ${MAX_FILE_BYTES / 1024 / 1024} MB.`);
  }
  const isPdf = file.type === 'application/pdf' || /\.pdf$/i.test(file.name);
  if (!isPdf) return { text: await file.text(), pages: null };

  const [pdfjs, worker] = await Promise.all([
    import('pdfjs-dist'),
    import('pdfjs-dist/build/pdf.worker.min.mjs?url'),
  ]);
  pdfjs.GlobalWorkerOptions.workerSrc = worker.default;
  const task = pdfjs.getDocument({
    data: new Uint8Array(await file.arrayBuffer()),
    // Served next to the page (see vite.config.ts); needed for many CJK fonts.
    cMapUrl: new URL('cmaps/', document.baseURI).href,
    cMapPacked: true,
  });
  const pdf = await task.promise;
  try {
    const pages: string[] = [];
    for (let n = 1; n <= pdf.numPages; n++) {
      const page = await pdf.getPage(n);
      const content = await page.getTextContent();
      pages.push(pageText(content.items.filter((item): item is TextRun & typeof item => 'str' in item)));
      page.cleanup();
      onPage?.(n, pdf.numPages);
    }
    return { text: joinPages(pages), pages: pdf.numPages };
  } finally {
    await task.destroy();
  }
}
