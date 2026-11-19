import { Component, computed, input, output } from '@angular/core';

import { Category } from '../../core/api/questions-api';

@Component({
  selector: 'app-category-sidebar',
  templateUrl: './category-sidebar.html',
  styleUrl: './category-sidebar.css',
})
export class CategorySidebar {
  readonly categories = input.required<Category[]>();
  readonly selected = input.required<number | null>();
  readonly state = input.required<'loading' | 'ready' | 'error'>();
  readonly changed = output<number | null>();
  readonly retry = output<void>();

  protected readonly total = computed(() =>
    this.categories().reduce((count, category) => count + category.question_count, 0),
  );
}
