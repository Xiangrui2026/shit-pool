import { readGroup, readMath } from './latex-syntax';

export function appendLatexTable(
  host: HTMLElement, source: string, columns: string,
  renderCell: (host: HTMLElement, source: string) => void,
): void {
  const document = host.ownerDocument;
  const scroll = document.createElement('div');
  scroll.className = 'latex-table-scroll';
  const table = document.createElement('table');
  const body = table.createTBody();
  const alignments = columns.match(/[lcr]/g) ?? [];

  for (const values of splitRows(source)) {
    const row = body.insertRow();
    let column = 0;
    for (const value of values) {
      const cell = row.insertCell();
      const merged = readMergedCell(value);
      const align = merged?.alignment ?? alignments[column];
      cell.style.textAlign = align === 'r' ? 'right' : align === 'c' ? 'center' : 'left';
      if (merged) cell.colSpan = merged.span;
      renderCell(cell, merged?.value ?? value);
      column += cell.colSpan;
    }
  }
  scroll.append(table);
  host.append(scroll);
}

function readMergedCell(value: string) {
  if (!value.startsWith('\\multicolumn')) return null;
  const span = readGroup(value, '\\multicolumn'.length);
  const columns = span && readGroup(value, span.end);
  const content = columns && readGroup(value, columns.end);
  if (!span || !columns || !content) return null;
  return {
    span: Math.min(100, Math.max(1, Number.parseInt(span.value, 10) || 1)),
    alignment: columns.value.match(/[lcr]/)?.[0],
    value: content.value + value.slice(content.end),
  };
}

function splitRows(source: string): string[][] {
  const controls = /\$\$?|\\(?:[a-zA-Z]+|[^\r\n])|[&{]/g;
  const rows: string[][] = [];
  let row: string[] = [];
  let cell = '';
  let offset = 0;

  for (let match = controls.exec(source); match; match = controls.exec(source)) {
    const token = match[0];
    const group = token === '{' ? readGroup(source, match.index) : readMath(source, controls.lastIndex, token);
    if (group) {
      controls.lastIndex = group.end;
      continue;
    }
    if (token !== '&' && token !== '\\\\' && token !== '\\hline' && token !== '\\cline') continue;

    cell += source.slice(offset, match.index);
    if (token === '&' || token === '\\\\') {
      row.push(cell.trim());
      cell = '';
    }
    if (token === '\\\\') {
      if (row.some(Boolean)) rows.push(row);
      row = [];
      controls.lastIndex = readGroup(source, controls.lastIndex, '[', ']')?.end ?? controls.lastIndex;
    } else if (token === '\\cline') {
      controls.lastIndex = readGroup(source, controls.lastIndex)?.end ?? controls.lastIndex;
    }
    offset = controls.lastIndex;
  }
  row.push((cell + source.slice(offset)).trim());
  if (row.some(Boolean)) rows.push(row);
  return rows;
}
