import katex from 'katex';
import 'katex/contrib/mhchem';

import { appendLatexImage, setRelativeWidth } from './latex-assets';
import { readEnvironment, readGroup, readMath } from './latex-syntax';
import { appendLatexTable } from './latex-table';

const textStyles: Record<string, string> = {
  '\\textbf': 'strong', '\\textit': 'em', '\\emph': 'em', '\\underline': 'u',
  '\\texttt': 'code', '\\shortstack': 'span',
};
const textSymbols: Record<string, string> = {
  '\\%': '%', '\\_': '_', '\\&': '&', '\\#': '#', '\\$': '$', '\\{': '{', '\\}': '}',
  '\\ ': ' ', '\\quad': '\u2003', '\\qquad': '\u2003\u2003', '\\ldots': '…', '\\AA': 'Å',
  '~': '\u00a0',
};
const layoutOnly = new Set([
  '\\noindent', '\\medskip', '\\smallskip', '\\bigskip', '\\small', '\\centering',
  '\\begingroup', '\\endgroup', '\\sloppy', '\\allowbreak',
]);
const environments = new Set([
  'center', 'minipage', 'tabular', 'equation', 'equation*', 'align', 'align*',
  'gather', 'gather*', 'verbatim', 'verbatim*', 'tikzpicture',
]);

export function appendLatexMarkup(host: HTMLElement, source: string): void {
  appendContent(host, source, 0);
}

function appendContent(host: HTMLElement, source: string, depth: number): void {
  if (depth > 32) {
    appendText(host, source);
    return;
  }
  const tokens = /\$\$?|\\(?:[a-zA-Z]+\*?|[^\r\n])|[~%]/g;
  let offset = 0;
  for (let match = tokens.exec(source); match; match = tokens.exec(source)) {
    appendText(host, source.slice(offset, match.index));
    offset = renderToken(host, source, match[0], tokens.lastIndex, depth);
    tokens.lastIndex = offset;
  }
  appendText(host, source.slice(offset));
}

function renderToken(host: HTMLElement, source: string, token: string, end: number, depth: number): number {
  const start = end - token.length;
  const document = host.ownerDocument;
  const math = readMath(source, end, token);
  if (math) {
    appendMath(host, math.value, token === '$$' || token === '\\[', source.slice(start, math.end));
    return math.end;
  }

  if (token === '\\begin') {
    const name = readGroup(source, end);
    if (name && environments.has(name.value)) {
      const next = renderEnvironment(host, source, start, name.value, name.end, depth);
      if (next !== null) return next;
    }
  }
  if (token === '\\includegraphics' || token === '\\includegraphics*') {
    const options = readGroup(source, end, '[', ']');
    const path = readGroup(source, options?.end ?? end);
    if (path && appendLatexImage(host, path.value, options?.value ?? '')) return path.end;
  }
  if (token === '\\tikz') {
    const raw = source.slice(start).split(/\r?\n/, 1)[0];
    appendCode(host, raw, true);
    return start + raw.length;
  }
  if (token === '\\verb' || token === '\\verb*') {
    const delimiter = source[end];
    const lineEnd = source.slice(end).search(/[\r\n]/);
    const close = delimiter && !/\s/.test(delimiter) ? source.indexOf(delimiter, end + 1) : -1;
    if (close >= 0 && (lineEnd < 0 || close < end + lineEnd)) {
      const code = document.createElement('code');
      code.textContent = source.slice(end + 1, close);
      host.append(code);
      return close + 1;
    }
  }
  if (token === '%') {
    const lineStart = source.lastIndexOf('\n', start - 1) + 1;
    if (!source.slice(lineStart, start).trim()) {
      const nextLine = source.indexOf('\n', end);
      return nextLine < 0 ? source.length : nextLine + 1;
    }
  }
  if (token === '\\par' || token === '\\\\' || token === '\\newline' || token === '\\linebreak') {
    if (host.hasChildNodes()) appendText(host, '\n');
    const hasOptions = token === '\\\\' || token === '\\linebreak';
    return hasOptions ? readGroup(source, end, '[', ']')?.end ?? end : end;
  }
  if (token === '\\nopagebreak') return readGroup(source, end, '[', ']')?.end ?? end;
  if (layoutOnly.has(token)) return end;
  if (token === '\\hfill') {
    appendText(host, ' ');
    return end;
  }
  if (textSymbols[token]) {
    appendText(host, textSymbols[token]);
    return end;
  }

  const group = readGroup(source, token === '\\shortstack' ? readGroup(source, end, '[', ']')?.end ?? end : end);
  if (group) {
    if (token === '\\ce' || token === '\\pu') {
      const expression = source.slice(start, group.end);
      appendMath(host, expression, false, expression);
      return group.end;
    }
    if (textStyles[token]) {
      const element = document.createElement(textStyles[token]);
      appendContent(element, group.value, depth + 1);
      host.append(element);
      return group.end;
    }
    if (token === '\\hspace' || token === '\\hspace*') {
      appendText(host, '\u2002');
      return group.end;
    }
    if (token === '\\textcolor' || token === '\\raisebox' || token === '\\setlength' || token === '\\rule') {
      const content = readGroup(source, group.end);
      if (content) {
        if (token === '\\textcolor' || token === '\\raisebox') appendContent(host, content.value, depth + 1);
        if (token === '\\rule' && group.value.trim() !== '0pt') appendText(host, '______');
        return content.end;
      }
    }
  }
  appendText(host, token);
  return end;
}

function renderEnvironment(
  host: HTMLElement, source: string, start: number, name: string, offset: number, depth: number,
): number | null {
  const hasArgument = name === 'tabular' || name === 'minipage';
  const options = hasArgument ? readGroup(source, offset, '[', ']') : null;
  const argument = hasArgument ? readGroup(source, options?.end ?? offset) : null;
  const body = readEnvironment(source, argument?.end ?? offset, name);
  if (!body) return null;

  if (name === 'tabular') {
    appendLatexTable(host, body.value, argument?.value ?? '', (cell, value) => appendContent(cell, value, depth + 1));
  } else if (name === 'center' || name === 'minipage') {
    const container = host.ownerDocument.createElement('div');
    container.className = 'latex-' + name;
    if (name === 'minipage' && argument) setRelativeWidth(container, argument.value);
    appendContent(container, body.value.trim(), depth + 1);
    host.append(container);
  } else if (name === 'tikzpicture' || name.startsWith('verbatim')) {
    appendCode(host, name === 'tikzpicture' ? source.slice(start, body.end) : body.value, name === 'tikzpicture');
  } else {
    const raw = source.slice(start, body.end);
    appendMath(host, name.startsWith('equation') ? body.value : raw, true, raw);
  }
  return body.end;
}

function appendCode(host: HTMLElement, value: string, tikz: boolean): void {
  const code = host.ownerDocument.createElement('pre');
  if (tikz) code.setAttribute('aria-label', 'TikZ 源码');
  code.textContent = value;
  host.append(code);
}

function appendText(host: HTMLElement, value: string): void {
  if (!value) return;
  const previous = host.lastChild;
  if (previous instanceof Text) {
    previous.appendData(previous.data.endsWith('\n') && value.startsWith('\n') ? value.slice(1) : value);
  } else {
    host.append(host.ownerDocument.createTextNode(value));
  }
}

function appendMath(host: HTMLElement, value: string, display: boolean, raw: string): void {
  const container = host.ownerDocument.createElement(display ? 'div' : 'span');
  try {
    katex.render(value, container, { displayMode: display, trust: false, maxExpand: 300, maxSize: 18 });
    host.append(container);
  } catch (error) {
    if (!(error instanceof katex.ParseError)) throw error;
    appendText(host, raw);
  }
}
