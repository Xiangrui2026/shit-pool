import { HttpErrorResponse } from '@angular/common/http';
import { Component, DestroyRef, computed, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';

import { PaperFile, RenderJob, RenderJobsApi } from '../../core/api/render-jobs-api';
import { LatexContent } from '../../shared/latex/latex-content';
import { PAPER_LIMIT, PaperDraft } from './paper-draft';

type JobView = RenderJob | 'missing' | 'unavailable';

const ERROR_LABELS: Record<string, string> = {
  render_disabled: '此站点尚未开启排版，可以继续在本地组卷。',
  render_queue_full: '排版队列已满，请稍后再试。',
  paper_question_unavailable: '有题目已不在题库中，请移除后重试。',
  invalid_paper_title: '请输入有效的卷名。',
  invalid_paper_questions: '题目列表无效，请检查选题。',
  missing_question_image: '有配图找不到，暂时无法编译。',
  unsupported_image_reference: '题目或解答含不支持的配图路径。',
  render_images_too_large: '试卷引用的图片太多或太大。',
  render_pdf_too_large: '生成的 PDF 超过大小限制。',
  render_timeout: '编译超时，请缩小试卷规模后重试。',
  queue_timeout: '排版等待超时，请重新送排。',
  render_interrupted: '排版执行中断，请重新送排。',
  render_template_error: '站点的 CChO 模板配置有误，请联系维护者。',
  render_engine_unavailable: '排版服务暂不可用，请稍后重试。',
  render_compile_failed: 'CChO 编译未通过，请检查题干和解答的 LaTeX，或联系站点维护者。',
};

@Component({
  selector: 'app-paper-composer',
  imports: [RouterLink, LatexContent],
  templateUrl: './paper-composer.html',
  styleUrl: './paper-composer.css',
})
export class PaperComposer {
  protected readonly draft = inject(PaperDraft);
  protected readonly limit = PAPER_LIMIT;
  protected readonly sending = signal(false);
  protected readonly submitError = signal('');
  protected readonly states = signal<Record<string, JobView>>({});
  protected readonly canSubmit = computed(() => {
    const { title, items } = this.draft.value();
    return title.trim().length > 0 && items.length > 0 && !this.sending();
  });
  private readonly api = inject(RenderJobsApi);
  private readonly lifecycle = inject(DestroyRef);
  private timer: ReturnType<typeof setTimeout> | undefined;
  private checking = false;

  constructor() {
    this.lifecycle.onDestroy(() => clearTimeout(this.timer));
    void this.refresh();
  }

  protected async submit(event: Event): Promise<void> {
    event.preventDefault();
    if (!this.canSubmit()) return;
    const submitted = this.draft.value();
    const sentTitle = submitted.title.trim();
    this.sending.set(true);
    this.submitError.set('');
    try {
      const job = await this.api.submit(sentTitle, submitted.items.map((item) => item.id));
      this.draft.recordSubmission({ id: job.id, title: sentTitle }, submitted);
      if (this.lifecycle.destroyed) return;
      this.states.update((states) => ({ ...states, [job.id]: job }));
      this.schedule();
    } catch (error) {
      if (!this.lifecycle.destroyed) this.submitError.set(this.errorLabel(
        error instanceof HttpErrorResponse ? error.error?.error : null,
        '未能确认送排结果，请稍后检查或重试。',
      ));
    } finally {
      if (!this.lifecycle.destroyed) this.sending.set(false);
    }
  }

  protected async refresh(): Promise<void> {
    if (this.checking || this.lifecycle.destroyed) return;
    this.checking = true;
    clearTimeout(this.timer);
    try {
      const jobs = this.draft.value().jobs.filter(({ id }) => {
        const state = this.states()[id];
        return state !== 'missing' && (typeof state !== 'object' || state.status !== 'failed');
      });
      await Promise.all(jobs.map(async ({ id }) => {
        let result: JobView;
        try {
          result = await this.api.status(id);
        } catch (error) {
          result = error instanceof HttpErrorResponse && error.status === 404 ? 'missing' : 'unavailable';
        }
        if (!this.lifecycle.destroyed && this.draft.value().jobs.some((job) => job.id === id)) {
          this.states.update((states) => ({ ...states, [id]: result }));
        }
      }));
    } finally {
      this.checking = false;
      this.schedule();
    }
  }

  protected label(state: JobView | undefined): string {
    if (!state) return '正在查询';
    if (state === 'missing') return '任务已过期或不存在';
    if (state === 'unavailable') return '暂时无法查询';
    return { queued: '排队中', running: '排版中', completed: '可下载', failed: '排版失败' }[state.status];
  }

  protected errorLabel(code: string | null, fallback = '排版未完成。'): string {
    return code && Object.hasOwn(ERROR_LABELS, code) ? ERROR_LABELS[code] : fallback;
  }

  protected download(id: string, file: PaperFile): string {
    return this.api.downloadUrl(id, file);
  }

  protected result(state: JobView | undefined): RenderJob | null {
    return typeof state === 'object' ? state : null;
  }

  private schedule(): void {
    clearTimeout(this.timer);
    if (this.lifecycle.destroyed) return;
    const pending = this.draft.value().jobs.some(({ id }) => {
      const state = this.states()[id];
      return !state || state === 'unavailable' || typeof state === 'object' && ['queued', 'running'].includes(state.status);
    });
    if (pending) this.timer = setTimeout(() => void this.refresh(), 3000);
  }
}
