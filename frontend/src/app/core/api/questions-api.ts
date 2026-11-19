import { HttpClient, HttpParams } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { firstValueFrom, timeout } from 'rxjs';

export interface Category {
  id: number;
  name: string;
  question_count: number;
}

export interface Tag {
  name: string;
  question_count: number;
}

export type TagMatch = 'any' | 'all';

export interface Question {
  id: number;
  category: string;
  source: string;
  original_number: number | null;
  paper_source: string;
  stem: string;
  answer: string;
  notes: string;
  tags: string;
}

export interface Solution {
  id: number;
  title: string;
  body: string;
  is_default: boolean;
}

export interface QuestionDetails extends Question {
  solutions?: Solution[];
}

export interface Paper {
  source: string;
  question_count: number;
}

export interface QueryFilters {
  keyword: string;
  category: number | null;
  tags: string[];
  tagMatch: TagMatch;
}

interface SearchPage {
  items: Question[];
  page: number;
  page_size: number;
  total: number;
}

@Injectable({ providedIn: 'root' })
export class QuestionsApi {
  private readonly http = inject(HttpClient);

  categories(): Promise<Category[]> {
    return firstValueFrom(this.http.get<Category[]>('/api/categories').pipe(timeout(6000)));
  }

  tags(): Promise<Tag[]> {
    return firstValueFrom(this.http.get<Tag[]>('/api/tags').pipe(timeout(6000)));
  }

  papers() {
    return this.http.get<Paper[]>('/api/papers').pipe(timeout(6000));
  }

  paperQuestions(source: string) {
    return this.http.get<Question[]>('/api/papers/questions', { params: { source } }).pipe(timeout(10000));
  }

  detail(id: number): Promise<QuestionDetails> {
    return firstValueFrom(this.http.get<QuestionDetails>('/api/questions/' + id).pipe(timeout(6000)));
  }

  export(id: number): Promise<Blob> {
    return firstValueFrom(this.http.get('/api/questions/' + id + '/export', { responseType: 'blob' }).pipe(timeout(20000)));
  }

  search(filters: QueryFilters, page: number): Promise<SearchPage> {
    let params = new HttpParams().set('page', page);
    if (filters.keyword) params = params.set('q', filters.keyword);
    if (filters.category !== null) params = params.set('category_id', filters.category);
    if (filters.tags.length) {
      params = params.set('tags', filters.tags.join(',')).set('tag_match', filters.tagMatch);
    }
    return firstValueFrom(this.http.get<SearchPage>('/api/questions', { params }).pipe(timeout(6000)));
  }
}
