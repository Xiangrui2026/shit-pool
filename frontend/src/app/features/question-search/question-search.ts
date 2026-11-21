import { Component, ElementRef, computed, inject, signal, viewChild } from '@angular/core';

import { Category, QueryFilters, Question, QuestionsApi, Tag, TagMatch } from '../../core/api/questions-api';
import { LatexContent } from '../../shared/latex/latex-content';
import { QuestionRating } from '../../shared/question-rating/question-rating';
import { CategorySidebar } from './category-sidebar';
import { QuestionDetail } from './question-detail';
import { TagFilter } from './tag-filter';

type FilterState = 'loading' | 'ready' | 'error';
type ResultState = 'loading' | 'ready' | 'error';

function defaultFilters(): QueryFilters {
  return { keyword: '', category: null, tags: [], tagMatch: 'any' };
}

@Component({
  selector: 'app-question-search',
  imports: [CategorySidebar, TagFilter, LatexContent, QuestionDetail, QuestionRating],
  templateUrl: './question-search.html',
  styleUrl: './question-search.css',
})
export class QuestionSearch {
  private readonly api = inject(QuestionsApi);
  protected readonly categories = signal<Category[]>([]);
  protected readonly tags = signal<Tag[]>([]);
  protected readonly categoryState = signal<FilterState>('loading');
  protected readonly tagState = signal<FilterState>('loading');
  protected readonly filters = signal(defaultFilters());
  protected readonly draft = signal('');
  protected readonly items = signal<Question[]>([]);
  protected readonly resultState = signal<ResultState>('loading');
  protected readonly page = signal(1);
  protected readonly pageSize = signal(10);
  protected readonly total = signal(0);
  protected readonly pageCount = computed(() => Math.ceil(this.total() / this.pageSize()));
  protected readonly visiblePages = computed(() => {
    const count = this.pageCount();
    const start = Math.max(1, Math.min(this.page() - 2, count - 4));
    return Array.from({ length: Math.min(count, 5) }, (_, index) => start + index);
  });
  private readonly resultsHeading = viewChild.required<ElementRef<HTMLElement>>('resultsHeading');
  protected readonly categoryLabel = computed(() =>
    this.categories().find((item) => item.id === this.filters().category)?.name ?? '全部分类',
  );
  protected readonly activeFilters = computed(() => {
    const { keyword, category, tags, tagMatch } = this.filters();
    return Boolean(keyword || category !== null || tags.length || tagMatch === 'all');
  });
  private requestId = 0;

  constructor() {
    void this.loadCategories();
    void this.loadTags();
    void this.runSearch();
  }

  protected async loadCategories(): Promise<void> {
    this.categoryState.set('loading');
    try {
      this.categories.set(await this.api.categories());
      this.categoryState.set('ready');
    } catch {
      this.categoryState.set('error');
    }
  }

  protected async loadTags(): Promise<void> {
    this.tagState.set('loading');
    try {
      this.tags.set(await this.api.tags());
      this.tagState.set('ready');
    } catch {
      this.tagState.set('error');
    }
  }

  protected submit(event: Event, keyword: string): void {
    event.preventDefault();
    this.apply({ keyword: keyword.trim() });
  }

  protected selectCategory(category: number | null): void {
    if (this.filters().category !== category) this.apply({ category });
  }

  protected toggleTag(tag: string): void {
    const selected = this.filters().tags;
    this.apply({ tags: selected.includes(tag) ? selected.filter((value) => value !== tag) : [...selected, tag] });
  }

  protected setTagMatch(tagMatch: TagMatch): void {
    if (this.filters().tags.length) {
      this.apply({ tagMatch });
    } else {
      this.filters.update((filters) => ({ ...filters, tagMatch }));
    }
  }

  protected clearTags(): void {
    this.apply({ tags: [] });
  }

  protected reset(keyword: HTMLInputElement): void {
    keyword.value = '';
    this.draft.set('');
    this.filters.set(defaultFilters());
    void this.runSearch();
  }

  protected goToPage(page: number): void {
    if (page < 1 || page > this.pageCount() || this.page() === page) return;

    const heading = this.resultsHeading().nativeElement;
    heading.focus({ preventScroll: true });
    heading.scrollIntoView({ block: 'start', behavior: 'instant' });
    void this.runSearch(page);
  }

  private apply(changes: Partial<QueryFilters>): void {
    this.filters.update((filters) => ({ ...filters, ...changes }));
    void this.runSearch();
  }

  protected async runSearch(page = 1): Promise<void> {
    const id = ++this.requestId;
    this.page.set(page);
    this.resultState.set('loading');
    this.items.set([]);
    try {
      const result = await this.api.search(this.filters(), page);
      if (id !== this.requestId) return;

      const lastPage = Math.max(1, Math.ceil(result.total / result.page_size));
      if (result.page > lastPage) {
        await this.runSearch(lastPage);
        return;
      }

      this.items.set(result.items);
      this.page.set(result.page);
      this.pageSize.set(result.page_size);
      this.total.set(result.total);
      this.resultState.set('ready');
    } catch {
      if (id === this.requestId) this.resultState.set('error');
    }
  }
}
