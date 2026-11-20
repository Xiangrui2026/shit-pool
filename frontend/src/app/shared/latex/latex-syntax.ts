interface Group {
  value: string;
  end: number;
}

const mathClosers: Record<string, string> = { '$': '$', '$$': '$$', '\\(': '\\)', '\\[': '\\]' };

export function readGroup(source: string, offset: number, left = '{', right = '}'): Group | null {
  const start = offset + (source.slice(offset).match(/^\s*/)?.[0].length ?? 0);
  if (source[start] !== left) return null;

  let depth = 1;
  for (let i = start + 1; i < source.length; i++) {
    if (source[i] === '\\') {
      i++;
    } else if (source[i] === left) {
      depth++;
    } else if (source[i] === right && --depth === 0) {
      return { value: source.slice(start + 1, i), end: i + 1 };
    }
  }
  return null;
}

export function readMath(source: string, offset: number, start: string): Group | null {
  const end = mathClosers[start];
  if (!end) return null;

  let depth = 0;
  for (let i = offset; i < source.length; i++) {
    if (depth === 0 && source.startsWith(end, i)) {
      return { value: source.slice(offset, i), end: i + end.length };
    }
    const char = source[i];
    if (char === '\\') i++;
    else if (char === '{') depth++;
    else if (char === '}' && depth > 0) depth--;
  }
  return null;
}

export function readEnvironment(source: string, offset: number, name: string): Group | null {
  if (name === 'verbatim' || name === 'verbatim*' || name === 'tikzpicture') {
    const end = source.indexOf('\\end{' + name + '}', offset);
    return end < 0
      ? { value: source.slice(offset), end: source.length }
      : { value: source.slice(offset, end), end: end + name.length + 6 };
  }

  const boundaries = /\\(begin|end)\s*\{([^{}]+)\}/g;
  boundaries.lastIndex = offset;
  let depth = 1;
  for (let boundary = boundaries.exec(source); boundary; boundary = boundaries.exec(source)) {
    if (boundary[2] !== name) continue;
    depth += boundary[1] === 'begin' ? 1 : -1;
    if (depth === 0) return { value: source.slice(offset, boundary.index), end: boundaries.lastIndex };
  }
  return null;
}
