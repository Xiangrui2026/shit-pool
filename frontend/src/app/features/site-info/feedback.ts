import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { firstValueFrom, timeout } from 'rxjs';

@Component({
  selector: 'app-feedback',
  templateUrl: './feedback.html',
  styleUrl: './site-info.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class Feedback {
  protected readonly message = signal('');
  protected readonly sending = signal(false);
  protected readonly notice = signal<{ kind: 'sent' | 'error'; text: string } | null>(null);
  private readonly http = inject(HttpClient);

  protected update(value: string): void {
    this.message.set(value);
    this.notice.set(null);
  }

  protected async submit(event: Event): Promise<void> {
    event.preventDefault();
    const message = this.message().trim();
    if (!message || this.sending()) return;

    this.sending.set(true);
    this.notice.set(null);
    try {
      await firstValueFrom(this.http.post<void>('/api/feedback', { message }).pipe(timeout(16000)));
      this.message.set('');
      this.notice.set({ kind: 'sent', text: '反馈已发送，谢谢！' });
    } catch (error) {
      const busy = error instanceof HttpErrorResponse && error.status === 429;
      this.notice.set({
        kind: 'error',
        text: busy ? '发送服务繁忙，请稍后重试。' : '暂时无法确认反馈已送出。内容已保留，请稍后重试。',
      });
    } finally {
      this.sending.set(false);
    }
  }
}
