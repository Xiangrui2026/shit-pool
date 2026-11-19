import { Injectable, signal } from '@angular/core';

import { Question } from '../../core/api/questions-api';

export const PAPER_LIMIT = 40;

interface DraftItem {
  id: number;
  source: string;
  stem: string;
}

interface SavedJob {
  id: string;
  title: string;
}

interface Draft {
  title: string;
  items: DraftItem[];
  jobs: SavedJob[];
}

const STORAGE_KEY = 'shit-pool.paper-draft.v1';
const JOB_LIMIT = 10;

function draftItem({ id, source, stem }: Question): DraftItem {
  return { id, source, stem };
}

@Injectable({ providedIn: 'root' })
export class PaperDraft {
  private readonly draft = signal<Draft>(this.restore());
  private readonly failed = signal(false);
  readonly value = this.draft.asReadonly();
  readonly storageFailed = this.failed.asReadonly();

  add(question: Question): void {
    const current = this.draft();
    if (current.items.length >= PAPER_LIMIT || current.items.some((item) => item.id === question.id)) return;
    this.save({ ...current, items: [...current.items, draftItem(question)] });
  }

  addPaper(source: string, questions: readonly Question[]): boolean {
    const current = this.draft();
    const paperItems = new Map(questions.map((question) => [question.id, draftItem(question)]));
    const otherItems = current.items.filter((item) => !paperItems.has(item.id));
    if (!paperItems.size || otherItems.length + paperItems.size > PAPER_LIMIT) return false;
    this.save({
      ...current,
      title: current.title.trim() ? current.title : Array.from(source).slice(0, 120).join(''),
      items: [...otherItems, ...paperItems.values()],
    });
    return true;
  }

  rename(title: string): void {
    this.save({ ...this.draft(), title });
  }

  remove(id: number): void {
    const current = this.draft();
    this.save({ ...current, items: current.items.filter((item) => item.id !== id) });
  }

  move(index: number, offset: -1 | 1): void {
    const current = this.draft();
    const items = [...current.items];
    const destination = index + offset;
    if (index < 0 || index >= items.length || destination < 0 || destination >= items.length) return;
    [items[index], items[destination]] = [items[destination], items[index]];
    this.save({ ...current, items });
  }

  recordSubmission(job: SavedJob, submitted: Pick<Draft, 'title' | 'items'>): void {
    const current = this.draft();
    const unchanged = current.title === submitted.title && current.items === submitted.items;
    this.save({
      title: unchanged ? '' : current.title,
      items: unchanged ? [] : current.items,
      jobs: [job, ...current.jobs.filter((item) => item.id !== job.id)].slice(0, JOB_LIMIT),
    });
  }

  forget(id: string): void {
    const current = this.draft();
    this.save({ ...current, jobs: current.jobs.filter((item) => item.id !== id) });
  }

  private save(draft: Draft): void {
    this.draft.set(draft);
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(draft));
      this.failed.set(false);
    } catch {
      this.failed.set(true);
    }
  }

  private restore(): Draft {
    try {
      const saved = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? 'null') as Partial<Draft> | null;
      if (saved && typeof saved.title === 'string' && Array.isArray(saved.items) && Array.isArray(saved.jobs)) {
        const items = saved.items.filter((item) => item && Number.isSafeInteger(item.id) && item.id > 0
          && typeof item.stem === 'string' && typeof item.source === 'string');
        const jobs = saved.jobs.filter((job) => job && /^[0-9a-f]{32}$/.test(job.id) && typeof job.title === 'string');
        return { title: saved.title, items: items.slice(0, PAPER_LIMIT), jobs: jobs.slice(0, JOB_LIMIT) };
      }
    } catch {}
    return { title: '', items: [], jobs: [] };
  }
}
