import { ChangeDetectionStrategy, Component, ElementRef, effect, inject, input } from '@angular/core';

import { appendLatexMarkup } from './latex-markup';

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
    effect(() => {
      const source = this.source();
      this.host.replaceChildren();
      appendLatexMarkup(this.host, source);
    });
  }
}
