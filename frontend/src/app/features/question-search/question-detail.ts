import { Component, ElementRef, computed, inject, signal, viewChild } from '@angular/core';

import { Question, QuestionsApi, Solution } from '../../core/api/questions-api';
import { LatexContent } from '../../shared/latex/latex-content';
import { QuestionRating } from '../../shared/question-rating/question-rating';
import { PAPER_LIMIT, PaperDraft } from '../compose/paper-draft';

@Component({
  selector: 'app-question-detail',
  imports: [LatexContent, QuestionRating],
  templateUrl: './question-detail.html',
  styleUrl: './question-detail.css',
})
export class QuestionDetail {
  private readonly dialog = viewChild.required<ElementRef<HTMLDialogElement>>('detail');
  private readonly api = inject(QuestionsApi);
  protected readonly draft = inject(PaperDraft);
  protected readonly question = signal<Question | null>(null);
  protected readonly questionId = signal<number | null>(null);
  protected readonly exporting = signal(false);
  protected readonly exportError = signal('');
  protected readonly solutions = signal<Solution[] | null>(null);
  protected readonly solutionError = signal(false);
  protected readonly addedToDraft = computed(() => this.draft.value().items.some((item) => item.id === this.question()?.id));
  protected readonly draftFull = computed(() => this.draft.value().items.length >= PAPER_LIMIT);
  private pendingDetail = 0;
  private pendingExport = 0;

  open(question: Question | number): void {
    const id = typeof question === 'number' ? question : question.id;
    this.questionId.set(id);
    this.question.set(typeof question === 'number' ? null : question);
    this.dialog().nativeElement.showModal();
    void this.loadSolutions(id);
  }

  protected async loadSolutions(id: number): Promise<void> {
    const request = ++this.pendingDetail;
    this.solutions.set(null);
    this.solutionError.set(false);
    try {
      const detail = await this.api.detail(id);
      if (request !== this.pendingDetail) return;
      this.question.set(detail);
      this.solutions.set(detail.solutions ?? (detail.answer.trim()
        ? [{ id: 0, title: '答案', body: detail.answer, is_default: true }]
        : []));
    } catch {
      if (request === this.pendingDetail) this.solutionError.set(true);
    }
  }

  protected close(): void {
    this.dialog().nativeElement.close();
  }

  protected closed(): void {
    this.pendingDetail++;
    this.pendingExport++;
    this.question.set(null);
    this.questionId.set(null);
    this.exporting.set(false);
    this.exportError.set('');
    this.solutions.set(null);
    this.solutionError.set(false);
  }

  protected async download(question: Question): Promise<void> {
    const request = ++this.pendingExport;
    this.exporting.set(true);
    this.exportError.set('');
    try {
      const file = await this.api.export(question.id);
      if (request !== this.pendingExport) return;
      if (file.type !== 'application/zip') throw new Error('Expected a ZIP archive');
      const url = URL.createObjectURL(file);
      const anchor = this.dialog().nativeElement.ownerDocument.createElement('a');
      anchor.href = url;
      anchor.download = 'question-' + question.id + '.zip';
      anchor.click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
    } catch {
      if (request === this.pendingExport) this.exportError.set('无法导出。请检查配图是否已上传，或稍后重试。');
    } finally {
      if (request === this.pendingExport) this.exporting.set(false);
    }
  }
}
