import { ChangeDetectionStrategy, Component, computed, inject, input, linkedSignal, signal } from '@angular/core';
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';

import { Vote } from '../../core/api/questions-api';
import { QuestionRatings } from './question-ratings';

@Component({
  selector: 'app-question-rating',
  templateUrl: './question-rating.html',
  styleUrl: './question-rating.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class QuestionRating {
  private readonly ratings = inject(QuestionRatings);
  readonly questionId = input.required<number>();
  readonly score = input.required<number>();

  protected readonly displayedScore = linkedSignal(() => this.score());
  protected readonly pending = computed(() => this.ratings.pendingIds().has(this.questionId()));
  protected readonly error = signal(false);

  constructor() {
    this.ratings.updated.pipe(takeUntilDestroyed()).subscribe(({ id, score }) => {
      if (id === this.questionId()) this.displayedScore.set(score);
    });
  }

  protected async vote(vote: Vote): Promise<void> {
    this.error.set(false);
    try {
      await this.ratings.vote(this.questionId(), vote);
    } catch {
      this.error.set(true);
    }
  }
}
