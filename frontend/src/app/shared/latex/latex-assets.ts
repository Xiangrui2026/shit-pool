const imagePath = /^images\/([0-9a-f]{64}\.(?:png|jpg))$/;

export function appendLatexImage(host: HTMLElement, path: string, options: string): boolean {
  const name = imagePath.exec(path.trim())?.[1];
  if (!name) return false;

  const image = host.ownerDocument.createElement('img');
  image.className = 'latex-image';
  image.alt = '题目配图';
  image.loading = 'lazy';
  image.decoding = 'async';
  const width = options.match(/(?:^|,)\s*width\s*=\s*([^,]+)/)?.[1];
  if (width) setRelativeWidth(image, width);
  image.src = '/api/images/' + name;
  host.append(image);
  return true;
}

export function setRelativeWidth(element: HTMLElement, length: string): void {
  const fraction = /^(?:(\d+(?:\.\d*)?|\.\d+)\s*)?\\(?:line|text)width$/.exec(length.trim());
  if (!fraction) return;
  const width = Number(fraction[1] ?? 1) * 100;
  if (width > 0) element.style.width = Math.min(width, 100) + '%';
}
