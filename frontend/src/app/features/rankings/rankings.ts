import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { toObservable, toSignal } from '@angular/core/rxjs-interop';
import { auditTime, catchError, distinctUntilChanged, map, merge, of, startWith, Subject, switchMap } from 'rxjs';

import { QuestionsApi, RankOrder, RankedQuestion, Tag, TagMatch } from '../../core/api/questions-api';
import { LatexContent } from '../../shared/latex/latex-content';
import { QuestionRating } from '../../shared/question-rating/question-rating';
import { QuestionRatings } from '../../shared/question-rating/question-ratings';
import { QuestionDetail } from '../question-search/question-detail';
import { TagFilter } from '../question-search/tag-filter';

interface RankingFilters {
  order: RankOrder;
  tags: string[];
  tagMatch: TagMatch;
}

type RankingState =
  | { status: 'loading' | 'error' }
  | { status: 'ready'; items: RankedQuestion[] };

@Component({
  selector: 'app-rankings',
  imports: [LatexContent, QuestionRating, QuestionDetail, TagFilter],
  templateUrl: './rankings.html',
  styleUrl: './rankings.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class Rankings {
  private readonly api = inject(QuestionsApi);
  private readonly votes = inject(QuestionRatings);
  protected readonly tags = signal<Tag[]>([]);
  protected readonly tagState = signal<'loading' | 'ready' | 'error'>('loading');
  protected readonly filters = signal<RankingFilters>({ order: 'good', tags: [], tagMatch: 'any' });
  protected readonly reloadRanking = new Subject<void>();
  protected readonly ranking = toSignal(
    toObservable(this.filters).pipe(
      distinctUntilChanged((previous, current) =>
        previous.order === current.order && previous.tags === current.tags
        && (!current.tags.length || previous.tagMatch === current.tagMatch),
      ),
      switchMap(({ order, tags, tagMatch }) =>
        merge(this.reloadRanking, this.votes.updated.pipe(auditTime(160))).pipe(
          startWith(null),
          switchMap(() => this.api.ranking(order, tags, tagMatch).pipe(
            map((items): RankingState => ({ status: 'ready', items })),
            catchError(() => of({ status: 'error' } as const)),
          )),
          startWith({ status: 'loading' } as const),
        ),
      ),
    ),
    { initialValue: { status: 'loading' } as const },
  );

  constructor() {
    void this.loadTags();
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

  protected toggleTag(tag: string): void {
    this.filters.update((filters) => ({
      ...filters,
      tags: filters.tags.includes(tag) ? filters.tags.filter((name) => name !== tag) : [...filters.tags, tag],
    }));
  }

  protected updateFilters(changes: Partial<RankingFilters>): void {
    this.filters.update((filters) => ({ ...filters, ...changes }));
  }
}
