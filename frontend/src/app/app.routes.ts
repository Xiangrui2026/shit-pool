import { Routes } from '@angular/router';

export const routes: Routes = [
  {
    path: '',
    pathMatch: 'full',
    title: 'SHIT-Pool 石库',
    loadComponent: () => import('./features/question-search/question-search').then((module) => module.QuestionSearch),
  },
  {
    path: 'papers',
    title: '套题 · SHIT-Pool 石库',
    loadComponent: () => import('./features/papers/paper-browser').then((module) => module.PaperBrowser),
  },
  {
    path: 'compose',
    title: '本地组卷 · SHIT-Pool 石库',
    loadComponent: () => import('./features/compose/paper-composer').then((module) => module.PaperComposer),
  },
  { path: '**', redirectTo: '' },
];
