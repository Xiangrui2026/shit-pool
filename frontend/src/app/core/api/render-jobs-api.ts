import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { firstValueFrom, timeout } from 'rxjs';

export type RenderStatus = 'queued' | 'running' | 'completed' | 'failed';
export type PaperFile = 'questions' | 'answers';

export interface RenderJob {
  id: string;
  status: RenderStatus;
  has_answers: boolean;
  error: string | null;
}

@Injectable({ providedIn: 'root' })
export class RenderJobsApi {
  private readonly http = inject(HttpClient);

  submit(title: string, questionIds: number[]): Promise<RenderJob> {
    return firstValueFrom(this.http.post<RenderJob>('/api/render-jobs', {
      title,
      question_ids: questionIds,
      template: 'ccho',
    }).pipe(timeout(10000)));
  }

  status(id: string): Promise<RenderJob> {
    return firstValueFrom(this.http.get<RenderJob>(this.jobUrl(id)).pipe(timeout(6000)));
  }

  downloadUrl(id: string, file: PaperFile): string {
    return this.jobUrl(id) + '/files/' + file;
  }

  private jobUrl(id: string): string {
    return '/api/render-jobs/' + encodeURIComponent(id);
  }
}
