import { Injectable, inject, signal } from '@angular/core';
import { Subject } from 'rxjs';

import { QuestionScore, QuestionsApi, Vote } from '../../core/api/questions-api';

@Injectable({ providedIn: 'root' })
export class QuestionRatings {
  private readonly api = inject(QuestionsApi);
  private readonly pending = signal<ReadonlySet<number>>(new Set());
  private readonly changes = new Subject<QuestionScore>();

  readonly pendingIds = this.pending.asReadonly();
  readonly updated = this.changes.asObservable();

  async vote(id: number, vote: Vote): Promise<void> {
    if (this.pending().has(id)) return;

    this.pending.update((ids) => new Set(ids).add(id));
    try {
      this.changes.next(await this.api.rate(id, vote));
    } finally {
      this.pending.update((ids) => {
        const remaining = new Set(ids);
        remaining.delete(id);
        return remaining;
      });
    }
  }
}
