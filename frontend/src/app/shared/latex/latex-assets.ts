const commands = /\\begin\{verbatim\}[\s\S]*?(?:\\end\{verbatim\}|$)|\\begin\{verbatim\*\}[\s\S]*?(?:\\end\{verbatim\*\}|$)|\\begin\{tikzpicture\}[\s\S]*?(?:\\end\{tikzpicture\}|$)|\\tikz\b[^\n]*|\\verb\*?([^a-zA-Z\s])|\\\\|\\%|%[^\r\n]*|\\includegraphics\*?\s*(?:\[([^[\]]*)\]\s*)?\{([^{}\r\n]+)\}/g;
const imagePath = /^images\/([0-9a-f]{64}\.(?:png|jpg))$/;

export function appendLatexAssets(host: HTMLElement, source: string): void {
  const document = host.ownerDocument;
  const scan = new RegExp(commands);
  let offset = 0;
  let match: RegExpExecArray | null;
  while ((match = scan.exec(source))) {
    const [raw, verbatim, options, path] = match;
    if (verbatim) {
      const rest = source.slice(scan.lastIndex).split(/[\r\n]/, 1)[0];
      const end = rest.indexOf(verbatim);
      scan.lastIndex += end < 0 ? rest.length : end + verbatim.length;
      continue;
    }
    const isTikz = raw.startsWith('\\tikz') || raw.startsWith('\\begin{tikzpicture}');
    if (!path && !isTikz) continue;

    host.append(document.createTextNode(source.slice(offset, match.index)));
    if (isTikz) {
      const code = document.createElement('pre');
      code.setAttribute('aria-label', 'TikZ 源码');
      code.textContent = raw;
      host.append(code);
    } else {
      const name = imagePath.exec(path.trim())?.[1];
      if (name) {
        const image = document.createElement('img');
        image.className = 'latex-image';
        image.alt = '题目配图';
        image.loading = 'lazy';
        image.decoding = 'async';
        const width = options?.match(/(?:^|,)\s*width\s*=\s*((?:\d+(?:\.\d*)?|\.\d+))?\s*\\(?:line|text)width\s*(?=,|$)/);
        if (width) {
          const percent = Number(width[1] ?? 1) * 100;
          if (percent > 0) image.style.width = Math.min(percent, 100) + '%';
        }
        image.src = '/api/images/' + name;
        host.append(image);
      } else {
        host.append(document.createTextNode(raw));
      }
    }
    offset = scan.lastIndex;
  }
  host.append(document.createTextNode(source.slice(offset)));
}
