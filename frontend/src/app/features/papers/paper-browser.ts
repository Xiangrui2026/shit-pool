import { Component, computed, inject, signal } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { ActivatedRoute, Router, RouterLink } from '@angular/router';
import { HttpErrorResponse } from '@angular/common/http';
import { catchError, combineLatest, distinctUntilChanged, map, of, startWith, Subject, switchMap } from 'rxjs';

import { Paper, Question, QuestionsApi } from '../../core/api/questions-api';
import { LatexContent } from '../../shared/latex/latex-content';
import { PAPER_LIMIT, PaperDraft } from '../compose/paper-draft';
import { QuestionDetail } from '../question-search/question-detail';

type LoadState<T> =
  | { status: 'loading' }
  | { status: 'ready'; data: T }
  | { status: 'error' | 'missing' };

@Component({
  selector: 'app-paper-browser',
  imports: [RouterLink, LatexContent, QuestionDetail],
  templateUrl: './paper-browser.html',
  styleUrl: './paper-browser.css',
})
export class PaperBrowser {
  private readonly api = inject(QuestionsApi);
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly draft = inject(PaperDraft);
  private readonly reloadPapers = new Subject<void>();
  private readonly reloadQuestions = new Subject<void>();

  protected readonly filter = signal('');
  protected readonly paperLimit = PAPER_LIMIT;
  protected readonly blockedSource = signal<string | null>(null);
  protected readonly catalog = toSignal(
    this.reloadPapers.pipe(
      startWith(undefined),
      switchMap(() => this.api.papers().pipe(
        map((data): LoadState<Paper[]> => ({ status: 'ready', data })),
        startWith({ status: 'loading' } as const),
        catchError(() => of({ status: 'error' } as const)),
      )),
    ),
    { initialValue: { status: 'loading' } },
  );
  private readonly sourceChanges = this.route.queryParamMap.pipe(
    map((params) => params.get('source')?.trim() || null),
    distinctUntilChanged(),
  );
  protected readonly source = toSignal(this.sourceChanges, { initialValue: null });
  protected readonly paper = toSignal(
    combineLatest([this.sourceChanges, this.reloadQuestions.pipe(startWith(undefined))]).pipe(
      switchMap(([source]) => source ? this.api.paperQuestions(source).pipe(
        map((data): LoadState<Question[]> => ({ status: 'ready', data })),
        startWith({ status: 'loading' } as const),
        catchError((error: unknown) => of({
          status: error instanceof HttpErrorResponse && error.status === 404 ? 'missing' : 'error',
        } as const)),
      ) : of(null)),
    ),
    { initialValue: null },
  );
  protected readonly visiblePapers = computed(() => {
    const catalog = this.catalog();
    if (catalog.status !== 'ready') return [];
    const term = this.filter().trim().toLowerCase();
    return catalog.data.filter((paper) => paper.source.toLowerCase().includes(term));
  });

  protected retryCatalog(): void {
    this.reloadPapers.next();
  }

  protected retryPaper(): void {
    this.reloadQuestions.next();
  }

  protected composePaper(source: string, questions: readonly Question[]): void {
    if (!this.draft.addPaper(source, questions)) {
      this.blockedSource.set(source);
      return;
    }
    void this.router.navigate(['/compose']);
  }
}
