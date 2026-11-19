declare module 'katex/contrib/mhchem';

declare module 'katex/contrib/auto-render' {
  import type { KatexOptions } from 'katex';

  interface AutoRenderOptions extends KatexOptions {
    delimiters: { left: string; right: string; display: boolean }[];
    errorCallback?: (message: string, error: Error) => void;
  }

  export default function renderMathInElement(element: HTMLElement, options: AutoRenderOptions): void;
}
