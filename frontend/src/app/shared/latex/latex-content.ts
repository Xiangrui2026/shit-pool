import { ChangeDetectionStrategy, Component, ElementRef, effect, inject, input } from '@angular/core';
import renderMathInElement from 'katex/contrib/auto-render';
import 'katex/contrib/mhchem';

import { appendLatexAssets } from './latex-assets';

@Component({
  selector: 'app-latex-content',
  template: '',
  styleUrl: './latex-content.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class LatexContent {
  readonly source = input.required<string>();
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef).nativeElement;

  constructor() {
    effect(() => this.render(this.source()));
  }

  private render(source: string): void {
    this.host.replaceChildren();
    appendLatexAssets(this.host, source);
    for (const node of Array.from(this.host.childNodes)) {
      if (node.nodeType === Node.TEXT_NODE) wrapChemistry(node);
    }

    renderMathInElement(this.host, {
      delimiters: [
        { left: '$$', right: '$$', display: true },
        { left: '\\[', right: '\\]', display: true },
        { left: '\\(', right: '\\)', display: false },
        { left: '$', right: '$', display: false },
        { left: '\\begin{equation}', right: '\\end{equation}', display: true },
        { left: '\\begin{align}', right: '\\end{align}', display: true },
        { left: '\\begin{align*}', right: '\\end{align*}', display: true },
        { left: '\\begin{gather}', right: '\\end{gather}', display: true },
        { left: '\\begin{gather*}', right: '\\end{gather*}', display: true },
      ],
      trust: false,
      maxExpand: 300,
      maxSize: 18,
      errorCallback: () => undefined,
    });
  }
}

function wrapChemistry(node: Node): void {
  const source = node.textContent ?? '';
  const tokens = /\\(?:\[|\]|\(|\)|%|\\)|%[^\r\n]*|\$\$?|(\\(?:ce|pu)\{[^{}\r\n]*(?:\{[^{}\r\n]*\}[^{}\r\n]*)*\})|\\(?:begin|end)\{(?:equation|align|gather)\*?\}/g;
  if (!source.includes('\\ce{') && !source.includes('\\pu{')) return;
  let mathEnd = '';
  const starts: Record<string, string> = { '$': '$', '$$': '$$', '\\(': '\\)', '\\[': '\\]' };
  node.textContent = source.replace(tokens, (token: string, chemistry?: string) => {
    if (chemistry) return mathEnd ? token : '\\(' + token + '\\)';
    if (token === mathEnd) mathEnd = '';
    else if (!mathEnd) mathEnd = starts[token] ?? (token.startsWith('\\begin') ? token.replace('\\begin', '\\end') : '');
    return token;
  });
}
