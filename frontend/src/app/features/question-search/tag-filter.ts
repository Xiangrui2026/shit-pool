import { Component, ElementRef, computed, input, output, signal, viewChild } from '@angular/core';

import { Tag, TagMatch } from '../../core/api/questions-api';

const SUGGESTION_LIMIT = 20;

@Component({
  selector: 'app-tag-filter',
  templateUrl: './tag-filter.html',
  styleUrl: './tag-filter.css',
})
export class TagFilter {
  readonly tags = input.required<Tag[]>();
  readonly selected = input.required<string[]>();
  readonly mode = input.required<TagMatch>();
  readonly state = input.required<'loading' | 'ready' | 'error'>();
  readonly toggled = output<string>();
  readonly modeChanged = output<TagMatch>();
  readonly cleared = output<void>();
  readonly retry = output<void>();

  protected readonly expanded = signal(false);
  protected readonly query = signal('');
  protected readonly activeIndex = signal(-1);
  private readonly suggestionList = viewChild<ElementRef<HTMLUListElement>>('suggestionList');
  protected readonly suggestions = computed(() => {
    if (this.state() !== 'ready') return [];

    const query = this.query().trim().toLowerCase();
    const selected = new Set(this.selected());
    const matches = this.tags().filter((tag) =>
      !selected.has(tag.name) && tag.name.toLowerCase().includes(query),
    );
    const exact = query ? matches.find((tag) => tag.name.toLowerCase() === query) : undefined;
    return (exact ? [exact, ...matches.filter((tag) => tag !== exact)] : matches).slice(0, SUGGESTION_LIMIT);
  });

  protected updateQuery(value: string): void {
    this.query.set(value);
    this.activeIndex.set(value.trim() ? 0 : -1);
    this.expanded.set(true);
    const list = this.suggestionList()?.nativeElement;
    if (list) list.scrollTop = 0;
  }

  protected chooseTag(tag: string, field: HTMLInputElement): void {
    this.toggled.emit(tag);
    this.query.set('');
    this.activeIndex.set(-1);
    field.focus({ preventScroll: true });
    this.expanded.set(false);
  }

  protected closeOnBlur(event: FocusEvent, picker: HTMLElement): void {
    if (!(event.relatedTarget instanceof Node) || !picker.contains(event.relatedTarget)) {
      this.expanded.set(false);
    }
  }

  protected handleKey(event: KeyboardEvent, field: HTMLInputElement): void {
    if (event.isComposing || event.keyCode === 229) return;

    switch (event.key) {
      case 'ArrowDown':
      case 'ArrowUp': {
        event.preventDefault();
        const count = this.suggestions().length;
        this.expanded.set(true);
        if (!count) return;
        const current = this.activeIndex();
        const index = event.key === 'ArrowDown'
          ? Math.min(current + 1, count - 1)
          : current <= 0 ? count - 1 : current - 1;
        this.activeIndex.set(index);
        this.suggestionList()?.nativeElement.children.item(index)?.scrollIntoView({ block: 'nearest' });
        break;
      }
      case 'Enter': {
        if (!this.expanded()) return;
        event.preventDefault();
        const tag = this.suggestions()[this.activeIndex()];
        if (tag) this.chooseTag(tag.name, field);
        break;
      }
      case 'Escape':
        if (!this.expanded()) return;
        event.preventDefault();
        this.expanded.set(false);
        break;
    }
  }
}
