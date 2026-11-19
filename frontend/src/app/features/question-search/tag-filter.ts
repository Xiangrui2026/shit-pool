import { Component, input, output } from '@angular/core';

import { Tag, TagMatch } from '../../core/api/questions-api';

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
}
